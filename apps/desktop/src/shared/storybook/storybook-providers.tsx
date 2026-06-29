import { QueryProvider } from "@/app/providers/query";
import { type ReactNode } from "react";

type StorybookProvidersProps = {
  children: ReactNode;
};

export function StorybookProviders({ children }: StorybookProvidersProps) {
  return <QueryProvider>{children}</QueryProvider>;
}
