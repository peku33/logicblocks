import { type Meta } from "@storybook/react-vite";
import { useState } from "react";
import { ButtonActionAsync, ButtonGroup, ButtonLink, ButtonPressRelease } from "./Button";

export default {
  title: "components/common/Button",
} satisfies Meta;

const onClickNull = () => Promise.resolve();

export const Basic: React.FC = () => {
  const [buttonPressReleaseState, setButtonPressReleaseState] = useState(false);

  return (
    <>
      <ButtonActionAsync active={false} onClick={onClickNull}>
        ButtonActionAsync inactive
      </ButtonActionAsync>
      <ButtonActionAsync active={true} onClick={onClickNull}>
        ButtonActionAsync Active
      </ButtonActionAsync>

      <ButtonPressRelease
        active={buttonPressReleaseState}
        onPressedChanged={(value) => {
          setButtonPressReleaseState(value);
        }}
      >
        ButtonPressRelease
      </ButtonPressRelease>

      <ButtonLink href="" targetBlank>
        Link button
      </ButtonLink>
      <ButtonGroup>
        <ButtonActionAsync active={false} onClick={onClickNull}>
          First
        </ButtonActionAsync>
        <ButtonActionAsync active={true} onClick={onClickNull}>
          Middle
        </ButtonActionAsync>
        <ButtonActionAsync active={false} onClick={onClickNull}>
          Last
        </ButtonActionAsync>
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
};
