import { Switch } from "@wealthfolio/ui/components/ui/switch";

export function SwitchRow({
  label,
  description,
  checked,
  onCheckedChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-start gap-2 text-sm">
      <Switch size="sm" className="mt-0.5" checked={checked} onCheckedChange={onCheckedChange} />
      <span className="min-w-0">
        <span className="block">{label}</span>
        {description && <span className="text-muted-foreground block text-xs">{description}</span>}
      </span>
    </label>
  );
}
