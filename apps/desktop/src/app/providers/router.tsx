import { createBrowserRouter, RouterProvider } from "react-router";
import { RepositoryPage } from "@/pages/repository";

const router = createBrowserRouter([
  {
    path: "/",
    element: <RepositoryPage />,
  },
]);

export function AppRouter() {
  return <RouterProvider router={router} />;
}
