import { createEffect, createSignal, JSX } from "solid-js";
import infoIcon from "../assets/img/info.png";
import { SelectOption } from "./FactorioSelect";

export interface TooltipProps {
	name: string;
	tooltip?: JSX.Element;
	options?: [string, string | SelectOption][];
	splash?: string;
}

export function Tooltip(props: TooltipProps) {
	const { name, options } = props;
	const [needsUpdate, setNeedsUpdate] = createSignal(true);
	let hasOptionTip = false;

	const optionSection = options?.map(([_, v]) => {
		if (typeof v === "string") return;
		hasOptionTip = true;

		return (
			<div class="my-0.75">
				<span class="font-semibold text-tan-500">{v.name}:</span> <span class="text-white font-light">{v.tooltip}</span>
			</div>
		);
	});

	createEffect(() => {
		if (!needsUpdate()) return;

		document.querySelectorAll(".tooltip-trigger").forEach((trigger) => {
			(trigger as HTMLElement).addEventListener("mousemove", (e) => {
				const tooltip = trigger.nextElementSibling! as HTMLElement;
				const rect = trigger.getBoundingClientRect();

				const vpWidth = window.innerWidth;
				const vpHeight = window.innerHeight;
				const tooltipRect = tooltip.getBoundingClientRect();

				let x = e.clientX + 10;
				let y = e.clientY + 10;

				// Check if tooltip would go off-screen to the right
				if (x + tooltipRect.width > vpWidth) {
					x = e.clientX - tooltipRect.width - 10;
				}

				// Check if tooltip would go off-screen at the bottom
				if (y + tooltipRect.height > vpHeight) {
					y = e.clientY - tooltipRect.height - 10;
				}

				tooltip.style.left = `${x}px`;
				tooltip.style.top = `${y}px`;
			});
		});

		setNeedsUpdate(false);
	});

	return (
		<>
			<img src={infoIcon} class="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info" />
			<div class="tooltip">
				<div class="tooltip-header">{name}</div>
				{props.tooltip && <div>{props.tooltip}</div>}
				{hasOptionTip && [<br />]}
				{optionSection}
				{props.splash && (
					<>
						<br />
						<div class="text-tan-500" style="opacity:0.6;">
							{props.splash}
						</div>
					</>
				)}
			</div>
		</>
	);
}
