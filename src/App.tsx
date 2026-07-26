import { ThemeProvider } from "@/components/theme-provider.tsx";
import { Toaster } from "@/components/ui/toast";
import { TooltipProvider } from "@/components/ui/tooltip";
import { router } from "./app/router.tsx";
import { RouterProvider } from "react-router";

function App() {
  return (
    <ThemeProvider defaultTheme="dark" storageKey="vite-ui-theme">
      <Toaster>
        <TooltipProvider>
          <RouterProvider router={router} />
        </TooltipProvider>
      </Toaster>
    </ThemeProvider>
  );
}

export default App;
