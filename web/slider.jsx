import { createSignal } from "solid-js";

export function Slider({ value, min, max, step, onChange, ref, ...props }) {
	const [currValue, setCurrValue] = createSignal(value);

	function handleChange(e) {
		const v = +e.target.value;
		if (v >= min && v <= max) {
			setCurrValue(v);
			onChange(v);
		}
	}

	return (
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-3">
				<div class="factorio-slider-wrapper min-w-full">
					<div
						class="factorio-slider-fill"
						style={{
							width: `${((currValue() - min) / (max - min)) * 100}%`,
						}}
					/>
					<input
						ref={ref}
						type="range"
						id={props.id}
						class="factorio-slider"
						value={currValue()}
						min={min}
						max={max}
						step={step}
						onInput={handleChange}
					/>
				</div>
				<input
					type="number"
					class="bg-gray-100 focus:bg-tan-500 min-w-24 px-3 py-2 border rounded focus:outline-none focus:ring"
					value={currValue()}
					min={min}
					max={max}
					step={step}
					onChange={handleChange}
					placeholder={`${min}-${max}`}
				/>
			</div>
		</div>
	);
}
