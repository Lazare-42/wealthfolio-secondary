import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { useEffect, useState } from "react";

import {
  Button,
  Icons,
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  Badge,
  Separator,
} from "@wealthfolio/ui";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
  FormDescription,
} from "@wealthfolio/ui/components/ui/form";
import { Input } from "@wealthfolio/ui/components/ui/input";

import { useReconciliationConfig, useUpdateReconciliationConfig } from "@/hooks/use-reconciliation";
import { useAccounts } from "@/hooks/use-accounts";
import { MappingFormDialog } from "./mapping-form-dialog";
import type { StatementAccountMapping } from "@/lib/types";

const settingsSchema = z.object({
  statementsDir: z.string().min(1, "Statements directory is required"),
  amountTolerance: z.coerce.number().min(0).optional(),
});

type SettingsFormValues = z.infer<typeof settingsSchema>;

interface ReconciliationSettingsSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ReconciliationSettingsSheet({
  open,
  onOpenChange,
}: ReconciliationSettingsSheetProps) {
  const { data: config, isLoading } = useReconciliationConfig();
  const updateConfig = useUpdateReconciliationConfig();
  const { accounts } = useAccounts();

  const [mappings, setMappings] = useState<StatementAccountMapping[]>([]);
  const [editingMapping, setEditingMapping] = useState<{
    index: number;
    mapping?: StatementAccountMapping;
  } | null>(null);

  const form = useForm<SettingsFormValues>({
    resolver: zodResolver(settingsSchema),
    defaultValues: {
      statementsDir: "",
      amountTolerance: 0.01,
    },
  });

  useEffect(() => {
    if (config) {
      form.reset({
        statementsDir: config.statementsDir ?? "",
        amountTolerance: config.amountTolerance ?? 0.01,
      });
      setMappings(config.mappings ?? []);
    }
  }, [config, form]);

  const onSubmit = (values: SettingsFormValues) => {
    updateConfig.mutate(
      {
        statementsDir: values.statementsDir,
        amountTolerance: values.amountTolerance,
        mappings,
      },
      { onSuccess: () => onOpenChange(false) },
    );
  };

  const handleSaveMapping = (mapping: StatementAccountMapping) => {
    if (editingMapping !== null && editingMapping.index >= 0) {
      setMappings((prev) => prev.map((m, i) => (i === editingMapping.index ? mapping : m)));
    } else {
      setMappings((prev) => [...prev, mapping]);
    }
    setEditingMapping(null);
  };

  const handleDeleteMapping = (index: number) => {
    setMappings((prev) => prev.filter((_, i) => i !== index));
  };

  const getAccountName = (accountId: string) => {
    return accounts?.find((a) => a.id === accountId)?.name ?? accountId;
  };

  if (isLoading) return null;

  return (
    <>
      <Sheet open={open} onOpenChange={onOpenChange}>
        <SheetContent className="w-full overflow-y-auto sm:max-w-lg">
          <SheetHeader className="px-6">
            <SheetTitle>Reconciliation Settings</SheetTitle>
          </SheetHeader>

          <div className="space-y-6 px-6 py-4">
            <Form {...form}>
              <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
                <FormField
                  control={form.control}
                  name="statementsDir"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Statements Directory</FormLabel>
                      <FormControl>
                        <Input placeholder="/path/to/statements" {...field} />
                      </FormControl>
                      <FormDescription>
                        Server-side directory path where CSV bank statements are stored.
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="amountTolerance"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Amount Tolerance</FormLabel>
                      <FormControl>
                        <Input type="number" step="0.01" placeholder="0.01" {...field} />
                      </FormControl>
                      <FormDescription>
                        Maximum difference for matching amounts (default: 0.01).
                      </FormDescription>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <Separator />

                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h3 className="text-sm font-medium">Account Mappings</h3>
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => setEditingMapping({ index: -1 })}
                    >
                      <Icons.Plus className="mr-1 h-3 w-3" />
                      Add Mapping
                    </Button>
                  </div>

                  {mappings.length === 0 ? (
                    <p className="text-muted-foreground py-4 text-center text-sm">
                      No mappings configured. Add a mapping to link file patterns to accounts.
                    </p>
                  ) : (
                    <div className="space-y-2">
                      {mappings.map((mapping, index) => (
                        <div
                          key={index}
                          className="bg-muted/50 flex items-center justify-between rounded-lg border p-3"
                        >
                          <div className="min-w-0 flex-1">
                            <p className="truncate text-sm font-medium">{mapping.filePattern}</p>
                            <p className="text-muted-foreground text-xs">
                              {getAccountName(mapping.accountId)}
                            </p>
                            <div className="mt-1 flex gap-1">
                              <Badge variant="outline" className="text-[10px]">
                                date: {mapping.fieldMappings.dateColumn}
                              </Badge>
                              <Badge variant="outline" className="text-[10px]">
                                amount: {mapping.fieldMappings.amountColumn}
                              </Badge>
                            </div>
                          </div>
                          <div className="flex shrink-0 items-center gap-1">
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              className="h-7 w-7"
                              onClick={() => setEditingMapping({ index, mapping })}
                            >
                              <Icons.Pencil className="h-3 w-3" />
                            </Button>
                            <Button
                              type="button"
                              variant="ghost"
                              size="icon"
                              className="text-destructive h-7 w-7"
                              onClick={() => handleDeleteMapping(index)}
                            >
                              <Icons.Trash className="h-3 w-3" />
                            </Button>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <Separator />

                <Button type="submit" className="w-full" disabled={updateConfig.isPending}>
                  {updateConfig.isPending && (
                    <Icons.Spinner className="mr-2 h-4 w-4 animate-spin" />
                  )}
                  Save Settings
                </Button>
              </form>
            </Form>
          </div>
        </SheetContent>
      </Sheet>

      <MappingFormDialog
        open={editingMapping !== null}
        onOpenChange={(open) => {
          if (!open) setEditingMapping(null);
        }}
        mapping={editingMapping?.mapping}
        onSave={handleSaveMapping}
      />
    </>
  );
}
