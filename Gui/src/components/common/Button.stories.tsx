import { Meta, Story } from "@storybook/react";
import { Button, ButtonGroup, ButtonLink } from "./Button";

export default {
  title: "components/common/Button",
} as Meta;

export const Basic: Story<{}> = () => (
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
  </>
);
