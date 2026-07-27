import { ThemeProvider } from "@/components/theme-provider.tsx";
import { Toaster } from "@/components/ui/toast";
import { TooltipProvider } from "@/components/ui/tooltip";
import { router } from "./app/router.tsx";
import { RouterProvider } from "react-router";
import type { ThemeMode } from "src/types/preferences";

function App({ initialTheme }: { initialTheme: ThemeMode }) {
  return (
    <ThemeProvider defaultTheme={initialTheme}>
      <Toaster>
        <TooltipProvider>
          <RouterProvider router={router} />
        </TooltipProvider>
      </Toaster>
    </ThemeProvider>
  );
}

export default App;
