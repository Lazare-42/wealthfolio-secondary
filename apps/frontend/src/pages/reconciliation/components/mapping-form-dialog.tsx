import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { useEffect, useState } from "react";

import {
  Button,
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Icons,
} from "@wealthfolio/ui";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@wealthfolio/ui/components/ui/form";
import { Input } from "@wealthfolio/ui/components/ui/input";

import { AccountSelector } from "@/components/account-selector";
import { AccountForm } from "@/pages/settings/accounts/components/account-form";
import { useAccounts } from "@/hooks/use-accounts";
import type { Account, StatementAccountMapping } from "@/lib/types";

const mappingSchema = z.object({
  filePattern: z.string().min(1, "File pattern is required"),
  accountId: z.string().min(1, "Account is required"),
  dateColumn: z.string().min(1, "Date column is required"),
  amountColumn: z.string().min(1, "Amount column is required"),
  descriptionColumn: z.string().optional(),
  typeColumn: z.string().optional(),
  defaultCurrency: z.string().optional(),
});

type MappingFormValues = z.infer<typeof mappingSchema>;

interface MappingFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mapping?: StatementAccountMapping;
  onSave: (mapping: StatementAccountMapping) => void;
}

export function MappingFormDialog({ open, onOpenChange, mapping, onSave }: MappingFormDialogProps) {
  const [showCreateAccount, setShowCreateAccount] = useState(false);
  const [selectedAccount, setSelectedAccount] = useState<Account | null>(null);

  const form = useForm<MappingFormValues>({
    resolver: zodResolver(mappingSchema),
    defaultValues: {
      filePattern: mapping?.filePattern ?? "",
      accountId: mapping?.accountId ?? "",
      dateColumn: mapping?.fieldMappings?.dateColumn ?? "Date",
      amountColumn: mapping?.fieldMappings?.amountColumn ?? "Amount",
      descriptionColumn: mapping?.fieldMappings?.descriptionColumn ?? "",
      typeColumn: mapping?.fieldMappings?.typeColumn ?? "",
      defaultCurrency: mapping?.fieldMappings?.defaultCurrency ?? "",
    },
  });

  const { accounts } = useAccounts();

  useEffect(() => {
    if (!open) return;
    form.reset({
      filePattern: mapping?.filePattern ?? "",
      accountId: mapping?.accountId ?? "",
      dateColumn: mapping?.fieldMappings?.dateColumn ?? "Date",
      amountColumn: mapping?.fieldMappings?.amountColumn ?? "Amount",
      descriptionColumn: mapping?.fieldMappings?.descriptionColumn ?? "",
      typeColumn: mapping?.fieldMappings?.typeColumn ?? "",
      defaultCurrency: mapping?.fieldMappings?.defaultCurrency ?? "",
    });
    const account = accounts.find((a) => a.id === mapping?.accountId) ?? null;
    setSelectedAccount(account);
  }, [mapping, open]); // eslint-disable-line react-hooks/exhaustive-deps

  const onSubmit = (values: MappingFormValues) => {
    onSave({
      filePattern: values.filePattern,
      accountId: values.accountId,
      fieldMappings: {
        dateColumn: values.dateColumn,
        amountColumn: values.amountColumn,
        descriptionColumn: values.descriptionColumn || undefined,
        typeColumn: values.typeColumn || undefined,
        defaultCurrency: values.defaultCurrency || undefined,
      },
    });
    onOpenChange(false);
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-[480px]">
          <DialogHeader>
            <DialogTitle>{mapping ? "Edit Mapping" : "Add Mapping"}</DialogTitle>
          </DialogHeader>
          <Form {...form}>
            <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
              <FormField
                control={form.control}
                name="filePattern"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>File Pattern</FormLabel>
                    <FormControl>
                      <Input placeholder="*checking*.csv" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="accountId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Account</FormLabel>
                    <FormControl>
                      <div className="flex items-center gap-2">
                        <div className="flex-1">
                          <AccountSelector
                            variant="form"
                            selectedAccount={selectedAccount}
                            setSelectedAccount={(account) => {
                              setSelectedAccount(account);
                              field.onChange(account.id);
                            }}
                          />
                        </div>
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={() => setShowCreateAccount(true)}
                        >
                          <Icons.Plus className="mr-1 h-3 w-3" />
                          New
                        </Button>
                      </div>
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <div className="space-y-3 rounded-lg border p-3">
                <p className="text-muted-foreground text-xs font-medium uppercase tracking-wide">
                  Column Mappings
                </p>
                <div className="grid grid-cols-2 gap-3">
                  <FormField
                    control={form.control}
                    name="dateColumn"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Date Column</FormLabel>
                        <FormControl>
                          <Input placeholder="Date" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="amountColumn"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Amount Column</FormLabel>
                        <FormControl>
                          <Input placeholder="Amount" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="descriptionColumn"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Description Column</FormLabel>
                        <FormControl>
                          <Input placeholder="Description" {...field} />
                        </FormControl>
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="typeColumn"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Type Column</FormLabel>
                        <FormControl>
                          <Input placeholder="Type" {...field} />
                        </FormControl>
                      </FormItem>
                    )}
                  />
                </div>
                <FormField
                  control={form.control}
                  name="defaultCurrency"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Default Currency</FormLabel>
                      <FormControl>
                        <Input placeholder="USD" {...field} />
                      </FormControl>
                    </FormItem>
                  )}
                />
              </div>

              <DialogFooter>
                <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                  Cancel
                </Button>
                <Button type="submit">Save Mapping</Button>
              </DialogFooter>
            </form>
          </Form>
        </DialogContent>
      </Dialog>

      <Dialog open={showCreateAccount} onOpenChange={setShowCreateAccount}>
        <DialogContent className="sm:max-w-[425px]">
          <AccountForm onSuccess={() => setShowCreateAccount(false)} />
        </DialogContent>
      </Dialog>
    </>
  );
}
