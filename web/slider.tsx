import { createSignal } from "solid-js";

type SliderProps = {
  value: number;
  disabled?: boolean;
  values?: number[];
  id?: string;
  min?: number;
  max?: number;
  step?: number;
  onSubmit(value: number): void;
  ref?(el: HTMLInputElement): void;
};

export function Slider(props: SliderProps) {
  let { value, values, min = 0, max = Infinity, step, onSubmit, ref } = props;

  const [currRawValue, setCurrRawValue] = createSignal(values ? values.indexOf(value) : value);
  const [currDisplayValue, setCurrDisplayValue] = createSignal(value);

  if (values) {
    min = 0;
    max = values.length - 1;
    step = undefined;
  }

  function handleValueChange(rawValue: number) {
    if (rawValue >= min && rawValue <= max) {
      setCurrRawValue(rawValue);
      setCurrDisplayValue(values ? values[rawValue] : rawValue);
    }
  }

  return (
    <div class="flex items-center justify-between w-full">
      <div class="flex justify-between items-center gap-3 w-full">
        <div aria-disabled={props.disabled} class="factorio-slider-wrapper min-w-45">
          <div
            class="factorio-slider-fill"
            aria-disabled={props.disabled}
            style={{ width: `${((currRawValue() - min) / (max - min)) * 100}%` }}
          />
          <input
            disabled={props.disabled}
            type="range"
            id={props.id}
            class="factorio-slider"
            value={currRawValue()}
            min={min}
            max={max}
            step={step}
            onInput={(e) => handleValueChange(+e.target.value)}
            onChange={(e) => {
              e.preventDefault();
              onSubmit(currDisplayValue());
            }}
          />
        </div>
        <input
          ref={ref}
          disabled={props.disabled}
          type="number"
          class="bg-gray-100 focus:bg-tan-500 min-w-24 px-3 py-2 border rounded focus:outline-none focus:ring"
          value={currDisplayValue()}
          min={values ? values[0] : min}
          max={values ? values[values.length - 1] : max}
          step={step}
          onInput={(e) => {
            const value = +e.target!.value;
            const input = e.currentTarget;

            if (values && !values.includes(value)) {
              input.setCustomValidity(`Value must be one of: ${values.join(", ")}`);
              input.setAttribute("aria-invalid", "true");
              input.reportValidity();

              return;
            }

            input.setCustomValidity("");
            input.setAttribute("aria-invalid", "false");

            handleValueChange(values ? values.indexOf(value) : value);
          }}
          placeholder={`${min}-${max}`}
        />
      </div>
    </div>
  );
}
