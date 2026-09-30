import infoIcon from "../assets/img/info.png";
import { SelectOption } from "./FactorioSelect";

export interface TooltipProps {
	name: string;
	tooltip?: string;
	options?: [string, SelectOption][];
	splash?: string;
}

export function Tooltip({ name, tooltip, options, splash }: TooltipProps) {
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

	return (
		<>
			<img src={infoIcon} class="inline-block ml-1 mb-0.5 w-4 h-4 tooltip-trigger" alt="Info" />
			<div class="tooltip">
				<div class="tooltip-header">{name}</div>
				{tooltip && <div>{tooltip}</div>}
				{hasOptionTip && [<br />]}
				{optionSection}
				{splash && (
					<>
						<br />
						<div class="text-tan-500" style="opacity:0.6;">
							{splash}
						</div>
					</>
				)}
			</div>
		</>
	);
}
