import { Meta } from "@storybook/react-vite";
import { Button, ButtonActionAsync, ButtonGroup, ButtonLink } from "./Button";

export default {
  title: "components/common/Button",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Button>Inactive button</Button>
    <Button active>Active button</Button>
    <ButtonLink href="">Link button</ButtonLink>
    <ButtonGroup>
      <Button>First</Button>
      <Button>Center</Button>
      <Button>Last</Button>
    </ButtonGroup>
    <ButtonGroup>
      <Button>First</Button>
      <Button active>Center very very large</Button>
      <Button>Last</Button>
    </ButtonGroup>
    <ButtonActionAsync
      active
      onClick={() =>
        new Promise<void>((resolve) => {
          setTimeout(resolve, 1000);
        })
      }
    >
      Action Button with 1s timeout
    </ButtonActionAsync>
  </>
);
