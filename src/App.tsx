import { ThemeProvider } from "@/components/theme-provider.tsx";
import { router } from "./app/router.tsx";
import { RouterProvider } from "react-router";

function App() {

  return (
    <>
      <ThemeProvider defaultTheme="dark" storageKey="vite-ui-theme">
        <RouterProvider router={router}/>
      </ThemeProvider>
    </>
  );
}

export default App;
