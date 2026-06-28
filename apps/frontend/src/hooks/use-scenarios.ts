import { createScenario, deleteScenario, getScenarios, updateScenarioEntry } from "@/adapters";
import { QueryKeys } from "@/lib/query-keys";
import type { NewPortfolioScenario, PortfolioScenario } from "@/lib/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

export function useScenarios() {
  return useQuery<PortfolioScenario[], Error>({
    queryKey: [QueryKeys.SCENARIOS],
    queryFn: getScenarios,
  });
}

export function useScenarioMutations() {
  const queryClient = useQueryClient();

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: [QueryKeys.SCENARIOS] });
  };

  const createMutation = useMutation({
    mutationFn: (scenario: NewPortfolioScenario) => createScenario(scenario),
    onSuccess: () => {
      invalidate();
      toast.success("Scenario created successfully.");
    },
    onError: () => toast.error("Failed to create scenario."),
  });

  const updateMutation = useMutation({
    mutationFn: (scenario: PortfolioScenario) => updateScenarioEntry(scenario),
    onSuccess: () => {
      invalidate();
      toast.success("Scenario updated successfully.");
    },
    onError: () => toast.error("Failed to update scenario."),
  });

  const deleteMutation = useMutation({
    mutationFn: (scenarioId: string) => deleteScenario(scenarioId),
    onSuccess: () => {
      invalidate();
      toast.success("Scenario deleted successfully.");
    },
    onError: () => toast.error("Failed to delete scenario."),
  });

  return { createMutation, updateMutation, deleteMutation };
}
