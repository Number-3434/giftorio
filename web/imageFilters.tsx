import { createSignal, For } from "solid-js";
import { DragDropProvider, DragDropSensors, SortableProvider, createSortable, closestCenter } from "@thisbeyond/solid-dnd";

const _FILTER_TYPES = {
	blur: {
		icon: "≋",
		name: "Blur",
		args: {
			sigma: {
				name: "sigma",
				default: 0.0,
			},
		},
	},
	brightness: {
		icon: "☀",
		name: "Brightness",
		args: {
			value: {
				name: "value",
				default: 0,
			},
		},
	},
	contrast: {
		icon: "◐",
		name: "Contrast",
		args: {
			value: {
				name: "value",
				default: 0,
			},
		},
	},
	huerotate: {
		icon: "⟳",
		name: "Hue Rotate",
		args: {
			value: {
				name: "value",
				default: 0,
			},
		},
	},
	unsharpen: {
		icon: "◌",
		name: "Unsharpen",
		args: {
			sigma: {
				name: "sigma",
				default: 0.0,
			},
			threshold: {
				name: "threshold",
				default: 0,
			},
		},
	},
} as const;

function defineFilters<const T extends Record<string, object>>(filters: T) {
	return Object.fromEntries(Object.entries(filters).map(([type, filter]) => [type, { type, ...filter }])) as {
		[K in keyof T]: T[K] & { type: K };
	};
}
const FILTER_TYPES = defineFilters(_FILTER_TYPES);

type FilterType = keyof typeof FILTER_TYPES;
type Filter = (typeof FILTER_TYPES)[FilterType];
type FilterInstance = {
	id: string;
	filter: Filter;
};

function SortableFilter({ filter, ...props }: { filter: FilterInstance }) {
	const sortable = createSortable(filter.id);
	const [showOptions, setShowOptions] = createSignal(false);

	function toggleOptions() {
		setShowOptions(!showOptions());
	}

	return (
		// @ts-ignore
		<div use:sortable class="factorio-tile-button" classList={{ "opacity-50": sortable.isActiveDraggable }} onClick={toggleOptions}>
			<span class="text-2xl cursor-grab filter-icon">{FILTER_TYPES[filter.filter.type].icon}</span>
			{showOptions() && (
				<div class="panel w-full h-full absolute">
					<div>Options</div>
					<For each={Object.entries(FILTER_TYPES[filter.filter.type].args)}>
						{([k, v]) => {
							return (
								<>
									{k}: {v.default}
								</>
							);
						}}
					</For>
				</div>
			)}
		</div>
	);
}

export default function ImageFilters({
	defaultFilters = [],
	onChange,
}: {
	defaultFilters?: FilterInstance[];
	onChange?(filters: FilterInstance[]): void;
}) {
	const [filters, setFilters] = createSignal<FilterInstance[]>(defaultFilters);
	const [selectedFilterType, setSelectedFilterType] = createSignal<FilterType>(Object.keys(FILTER_TYPES)[0] as FilterType);

	let nextFilterId = Math.max(-1, ...defaultFilters.map((v) => +v.id)) + 1;

	function addFilter(type: FilterType) {
		setFilters((prev) => [...prev, { id: `${nextFilterId++}`, filter: FILTER_TYPES[type] }]);
		onChange?.(filters());
	}

	function reorder(fromId: string, toId: string) {
		setFilters((prev) => {
			const from = prev.findIndex((f) => f.id === fromId);
			const to = prev.findIndex((f) => f.id === toId);
			if (from === -1 || to === -1 || from === to) return prev;
			const result = prev.slice();
			result.splice(to, 0, ...result.splice(from, 1));
			return result;
		});
		onChange?.(filters());
	}

	return (
		<DragDropProvider
			collisionDetector={closestCenter}
			onDragEnd={({ draggable, droppable }) => {
				if (!draggable || !droppable) return;
				reorder(draggable.id as string, droppable.id as string);
			}}
		>
			<DragDropSensors />
			<div class="flex-col gap-2 p-2 max-w-md">
				<div class="flex factorio-tile-container">
					<SortableProvider ids={filters().map((f) => f.id)}>
						<For each={filters()}>{(f) => <SortableFilter filter={f} />}</For>
					</SortableProvider>
				</div>
				<div class="flex gap-2 p-2 factorio-select-container factorio-form-element">
					<select value={selectedFilterType()} onChange={(e) => setSelectedFilterType(e.currentTarget.value as FilterType)}>
						<For each={Object.entries(FILTER_TYPES)}>{([type, filter]) => <option value={type}>{filter.name}</option>}</For>
					</select>
				</div>
				<button class="button" type="button" onClick={() => addFilter(selectedFilterType()!)}>
					Add
				</button>
			</div>
		</DragDropProvider>
	);
}
