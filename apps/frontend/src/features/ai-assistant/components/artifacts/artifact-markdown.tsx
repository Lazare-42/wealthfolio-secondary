import { memo, type ComponentPropsWithoutRef } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";

import { ExternalLink } from "@/components/external-link";
import { cn } from "@/lib/utils";

/**
 * Standalone markdown renderer for `kind: "report"` artifacts. Mirrors the
 * chat's `MarkdownText` styling (prose + bordered GFM tables) but renders an
 * arbitrary string instead of an assistant-ui message part.
 */
const components: Components = {
  a: ({ href, children, className, ...props }: ComponentPropsWithoutRef<"a">) => (
    <ExternalLink
      href={href ?? "#"}
      className={cn("text-primary hover:text-primary/80 underline underline-offset-2", className)}
      {...props}
    >
      {children}
    </ExternalLink>
  ),
  pre: ({ className, ...props }: ComponentPropsWithoutRef<"pre">) => (
    <pre
      className={cn(
        "not-prose overflow-x-auto rounded-lg bg-black p-4 text-sm text-white",
        className,
      )}
      {...props}
    />
  ),
  code: ({ className, ...props }: ComponentPropsWithoutRef<"code">) => (
    <code
      className={cn(
        "bg-muted rounded border px-1.5 py-0.5 font-mono text-sm font-medium before:content-none after:content-none",
        className,
      )}
      {...props}
    />
  ),
  table: ({ className, ...props }: ComponentPropsWithoutRef<"table">) => (
    <div className="overflow-x-auto">
      <table
        className={cn("not-prose my-3 w-full border-separate border-spacing-0 text-sm", className)}
        {...props}
      />
    </div>
  ),
  th: ({ className, ...props }: ComponentPropsWithoutRef<"th">) => (
    <th
      className={cn(
        "bg-muted whitespace-nowrap px-3 py-2 text-left text-xs font-semibold first:rounded-tl-lg last:rounded-tr-lg",
        className,
      )}
      {...props}
    />
  ),
  td: ({ className, ...props }: ComponentPropsWithoutRef<"td">) => (
    <td
      className={cn(
        "whitespace-nowrap border-b border-l px-3 py-2 text-left text-sm last:border-r",
        className,
      )}
      {...props}
    />
  ),
};

function ArtifactMarkdownImpl({ markdown }: { markdown: string }) {
  return (
    <div className="prose prose-sm dark:prose-invert prose-p:leading-relaxed max-w-none">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {markdown}
      </ReactMarkdown>
    </div>
  );
}

export const ArtifactMarkdown = memo(ArtifactMarkdownImpl);
