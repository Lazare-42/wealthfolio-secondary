import { Switch } from "@wealthfolio/ui/components/ui/switch";

export function SwitchRow({
  label,
  checked,
  onCheckedChange,
}: {
  label: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm">
      <Switch size="sm" checked={checked} onCheckedChange={onCheckedChange} />
      {label}
    </label>
  );
}
