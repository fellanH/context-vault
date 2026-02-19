import { createBrowserRouter } from "react-router";
import { MarketingLayout } from "./components/MarketingLayout";
import { LandingPage } from "./pages/LandingPage";
import { BlogIndexPage } from "./pages/BlogIndexPage";
import { BlogPostPage } from "./pages/BlogPostPage";

export const router = createBrowserRouter([
  {
    path: "/",
    Component: MarketingLayout,
    children: [
      { index: true, Component: LandingPage },
      { path: "blog", Component: BlogIndexPage },
      { path: "blog/:slug", Component: BlogPostPage },
    ],
  },
]);
