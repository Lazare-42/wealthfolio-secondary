import {
  Children,
  createContext,
  isValidElement,
  memo,
  useContext,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from "react";
import ReactMarkdown, { type Components, type ExtraProps } from "react-markdown";
import remarkGfm from "remark-gfm";

import { cn } from "@/lib/utils";
import { CodeHeader, MarkdownLink, markdownClasses } from "../markdown-text";

/**
 * Standalone markdown renderer for `kind: "report"` artifacts. Reuses the
 * chat's `MarkdownText` link handling and class strings (`markdownClasses`)
 * but renders an arbitrary string instead of an assistant-ui message part.
 */

/**
 * True while rendering inside a fenced code block. Plain `react-markdown` has
 * no equivalent of assistant-ui's `useIsMarkdownCodeBlock`, so the `pre`
 * renderer provides it for its children.
 */
const CodeBlockContext = createContext(false);

/** Pull the language + raw code out of a code block's `<code>` child (for the copy header). */
function extractCodeBlock(children: ReactNode): { language: string | undefined; code: string } {
  let language: string | undefined;
  let code = "";
  Children.forEach(children, (child) => {
    if (!isValidElement<{ className?: string; children?: ReactNode }>(child)) return;
    language ??= /language-(\S+)/.exec(child.props.className ?? "")?.[1];
    if (typeof child.props.children === "string") code += child.props.children;
  });
  return { language, code };
}

const components: Components = {
  a: ({ node: _node, ...props }: ComponentPropsWithoutRef<"a"> & ExtraProps) => (
    <MarkdownLink {...props} />
  ),
  pre: ({
    node: _node,
    className,
    children,
    ...props
  }: ComponentPropsWithoutRef<"pre"> & ExtraProps) => {
    const { language, code } = extractCodeBlock(children);
    return (
      <CodeBlockContext.Provider value={true}>
        <CodeHeader language={language} code={code} />
        <pre className={cn(markdownClasses.pre, className)} {...props}>
          {children}
        </pre>
      </CodeBlockContext.Provider>
    );
  },
  code: function Code({
    node: _node,
    className,
    ...props
  }: ComponentPropsWithoutRef<"code"> & ExtraProps) {
    const isCodeBlock = useContext(CodeBlockContext);
    return (
      <code className={cn(!isCodeBlock && markdownClasses.inlineCode, className)} {...props} />
    );
  },
  table: ({ node: _node, className, ...props }: ComponentPropsWithoutRef<"table"> & ExtraProps) => (
    <div className="overflow-x-auto">
      <table className={cn(markdownClasses.table, className)} {...props} />
    </div>
  ),
  th: ({ node: _node, className, ...props }: ComponentPropsWithoutRef<"th"> & ExtraProps) => (
    <th className={cn(markdownClasses.th, className)} {...props} />
  ),
  td: ({ node: _node, className, ...props }: ComponentPropsWithoutRef<"td"> & ExtraProps) => (
    <td className={cn(markdownClasses.td, className)} {...props} />
  ),
  tr: ({ node: _node, className, ...props }: ComponentPropsWithoutRef<"tr"> & ExtraProps) => (
    <tr className={cn(markdownClasses.tr, className)} {...props} />
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
