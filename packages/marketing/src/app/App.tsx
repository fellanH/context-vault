import { RouterProvider } from "react-router";
import { ThemeProvider } from "next-themes";
import { router } from "./routes";

function App() {
  return (
    <ThemeProvider attribute="class" defaultTheme="light" enableSystem>
      <RouterProvider router={router} />
    </ThemeProvider>
  );
}

export default App;
