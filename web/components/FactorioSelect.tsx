import { createEffect, createSignal, For, JSX } from "solid-js";
import { FormElementValue } from "../types";
import { Tooltip } from "./Tooltip";

export type SelectOption = { name: string; tooltip?: string };
export interface FactorioSelectProps extends Omit<JSX.SelectHTMLAttributes<HTMLSelectElement>, "onChange"> {
  children?: JSX.Element;
  formArgs?: FormElementValue;
  initialValue: string;
  key: string;
  name: string;
  onChange(value: string): void;
  tooltip?: string;
  options: [string, string | SelectOption][];
  splash?: string;
  ref(el: HTMLSelectElement): void;
}

export function FactorioSelect(props: FactorioSelectProps) {
  const { formArgs, initialValue, key, name, onChange, options, tooltip, splash, ref, ...selectProps } = props;
  const [value, setValue] = createSignal(initialValue);
  const [selectTitle, setSelectTitle] = createSignal<string>();

  function handleChange(e: Event) {
    const target = e.currentTarget as HTMLSelectElement;
    setValue(target.value);
    onChange(target.value);
  }

  createEffect(() => {
    const currValue = value();
    const option = options.find((v) => (typeof v === "string" ? v : v[0] === currValue));
    if (!option) return;

    const [_, info] = option;

    if (typeof info === "string") {
      setSelectTitle(tooltip);
    } else {
      setSelectTitle(`${tooltip}\n\n${info.name}: ${info.tooltip}`);
    }
  });

  return (
    <div
      class="mt-1 mb-1 flex items-center justify-between factorio-form-element factorio-select-container"
      aria-disabled={formArgs?.disabled}
    >
      <label class="block text-white-500" for={key}>
        {name}
        <Tooltip name={name} tooltip={tooltip} splash={splash} options={options} />
      </label>
      <select ref={ref} id={key} name={key} value={value()} onChange={handleChange} title={selectTitle()} {...selectProps}>
        <For each={options}>
          {([k, v]) => (
            <option title={typeof v === "string" ? undefined : v.tooltip} value={k}>
              {typeof v === "string" ? v : v.name}
            </option>
          )}
        </For>
        {props.children}
      </select>
    </div>
  );
}
