import type { Preview } from "@storybook/react";
import GlobalStyles from "../src/GlobalStyles";

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
  decorators: [
    (Story) => (
      <>
        <GlobalStyles />
        <div style={{ margin: "0.25rem" }}>
          <Story />
        </div>
      </>
    ),
  ],
};

export default preview;
