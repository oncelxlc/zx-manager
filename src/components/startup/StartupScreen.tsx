import { TerminalSquareIcon } from "lucide-react";

import { Spinner } from "@/components/ui/spinner";

export function StartupScreen() {
  return (
    <main
      aria-busy="true"
      className="flex min-h-svh items-center justify-center bg-background px-6 text-foreground"
      data-testid="startup-screen"
    >
      <div className="flex flex-col items-center gap-5 text-center">
        <div className="flex size-12 items-center justify-center rounded-full bg-primary text-primary-foreground">
          <TerminalSquareIcon aria-hidden="true" />
        </div>
        <h1 className="text-xl font-semibold tracking-tight">ZxManager</h1>
        <Spinner aria-hidden="true" className="size-6 text-muted-foreground" />
      </div>
    </main>
  );
}
