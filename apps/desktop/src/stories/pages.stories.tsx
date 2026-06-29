import type { Meta, StoryObj } from "@storybook/react-vite";
import { RepositoryPage } from "@/pages/repository";
import { StorybookProviders } from "@/shared/storybook/storybook-providers";

const meta = {
  title: "Pages/Repository",
  component: RepositoryPage,
  decorators: [
    (Story) => (
      <StorybookProviders>
        <Story />
      </StorybookProviders>
    ),
  ],
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta<typeof RepositoryPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
