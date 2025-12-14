import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Icons } from "@/components/ui/icons";
import { Progress } from "@/components/ui/progress";

interface AiParsingOverlayProps {
  visible: boolean;
}

export function AiParsingOverlay({ visible }: AiParsingOverlayProps) {
  if (!visible) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <Card className="w-[400px]">
        <CardHeader>
          <div className="flex items-center gap-2">
            <Icons.Sparkles className="h-5 w-5 animate-pulse text-primary" />
            <CardTitle>AI Processing Document</CardTitle>
          </div>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Icons.Loader className="h-4 w-4 animate-spin" />
              <span className="text-sm">Analyzing document structure...</span>
            </div>
            <Progress value={33} />
          </div>

          <p className="text-xs text-muted-foreground">
            This may take 10-30 seconds depending on document complexity
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
