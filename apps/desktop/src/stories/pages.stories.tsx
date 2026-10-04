import type { Meta, StoryObj } from "@storybook/react-vite";
import { RepositoryPage } from "@/pages/repository";

const meta = {
  title: "Pages/Repository",
  component: RepositoryPage,
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta<typeof RepositoryPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
