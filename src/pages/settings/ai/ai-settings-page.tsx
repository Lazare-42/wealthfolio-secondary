import { Separator } from "@/components/ui/separator";
import { SettingsHeader } from "../settings-header";
import { ProviderSettings } from "./components/provider-settings";
import { CapabilityMatrix } from "./components/capability-matrix";

export default function AISettingsPage() {
  return (
    <div className="space-y-6">
      <SettingsHeader
        heading="AI & Automation"
        text="Configure AI providers for intelligent document parsing and CSV mapping."
      />
      <Separator />
      <ProviderSettings />
      <div className="pt-6">
        <CapabilityMatrix />
      </div>
    </div>
  );
}
