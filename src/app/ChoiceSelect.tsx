import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from "../components/ui/select";

interface ChoiceSelectProps<T extends string> {
  value: T;
  onValueChange: (value: T) => void;
  options: ReadonlyArray<{ value: T; label: string }>;
  id?: string;
  "aria-label"?: string;
  "aria-describedby"?: string;
  disabled?: boolean;
  size?: "sm" | "default";
}

export function ChoiceSelect<T extends string>({ value, options, onValueChange, disabled, ...trigger }: ChoiceSelectProps<T>) {
  return <Select<T> items={options} value={value} disabled={disabled} onValueChange={next => {
    if (next !== null) onValueChange(next);
  }}>
    <SelectTrigger {...trigger}><SelectValue /></SelectTrigger>
    <SelectContent>
      <SelectGroup>{options.map(option =>
        <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
      )}</SelectGroup>
    </SelectContent>
  </Select>;
}
