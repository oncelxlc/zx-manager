import type { ReactNode } from "react";
import { Link, useSearchParams } from "react-router";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs";

export function ResourceTabs<T extends string>({ label, tabs, current, children }: {
  label: string; tabs: ReadonlyArray<readonly [T, string]>; current: T; children: ReactNode;
}) {
  const [params] = useSearchParams();
  return <Tabs value={current}>
    <TabsList aria-label={label} activateOnFocus={false}>
      {tabs.map(([value, text]) => {
        const target = new URLSearchParams(params);
        target.set("tab", value);
        return <TabsTrigger key={value} value={value} nativeButton={false}
          render={<Link to={{ search: target.toString() }} />}>{text}</TabsTrigger>;
      })}
    </TabsList>
    <TabsContent value={current}>{children}</TabsContent>
  </Tabs>;
}
