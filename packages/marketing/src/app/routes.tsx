import { createBrowserRouter } from "react-router";
import { MarketingLayout } from "./components/MarketingLayout";
import { LandingPage } from "./pages/LandingPage";
import { GetStartedPage } from "./pages/GetStartedPage";
import { BlogIndexPage } from "./pages/BlogIndexPage";
import { BlogPostPage } from "./pages/BlogPostPage";

export const router = createBrowserRouter([
  {
    path: "/",
    Component: MarketingLayout,
    children: [
      { index: true, Component: LandingPage },
      { path: "get-started", Component: GetStartedPage },
      { path: "blog", Component: BlogIndexPage },
      { path: "blog/:slug", Component: BlogPostPage },
    ],
  },
]);
