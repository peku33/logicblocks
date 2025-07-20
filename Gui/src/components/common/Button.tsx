import { type MouseEvent, type TouchEvent, type UIEvent, useCallback, useRef, useState } from "react";
import styled, { css } from "styled-components";
import Colors from "./Colors";

const ButtonBase = css`
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  text-align: center;

  padding: 0.5rem 1rem;
  border-radius: 0.25rem;

  font-weight: bold;
  cursor: pointer;

  background-color: ${Colors.GREY};
  color: ${Colors.WHITE};

  &:hover {
    background-color: ${Colors.GREY_DARK};
  }
`;
const ButtonActiveMixin = css`
  background-color: ${Colors.GREEN};

  &:hover {
    background-color: ${Colors.GREEN};
  }
`;

export const ButtonActionAsync: React.FC<{
  active: boolean;
  onClick: () => Promise<void>;
  children?: React.ReactNode;
}> = (props) => {
  const { active, onClick, children } = props;

  const [pending, setPending] = useState(false);

  function onClickWrapper() {
    if (pending) return;

    (async () => {
      setPending(true);
      await onClick();
      setPending(false);
    })().catch((reason: unknown) => {
      console.error(reason);
    });
  }

  return (
    <ButtonActionAsyncInner $active={pending ? false : active} onClick={onClickWrapper}>
      {children}
    </ButtonActionAsyncInner>
  );
};
// TODO: support undefined
const ButtonActionAsyncInner = styled.div<{
  $active: boolean;
}>`
  ${ButtonBase}
  ${({ $active }) => ($active ? ButtonActiveMixin : null)}
`;

export const ButtonPressRelease: React.FC<{
  active: boolean;
  onPressedChanged: (pressed: boolean) => void;
  children?: React.ReactNode;
}> = (props) => {
  const { active, onPressedChanged, children } = props;

  const pressed = useRef(false);
  const onEvent = useCallback(
    (event: UIEvent<HTMLDivElement>, value: boolean) => {
      event.preventDefault();

      if (value == pressed.current) {
        return;
      }
      onPressedChanged(value);
      pressed.current = value;
    },
    [onPressedChanged],
  );

  const onMouseDown = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      onEvent(event, true);
    },
    [onEvent],
  );
  const onMouseUp = useCallback(
    (event: MouseEvent<HTMLDivElement>) => {
      onEvent(event, false);
    },
    [onEvent],
  );
  const onTouchStart = useCallback(
    (event: TouchEvent<HTMLDivElement>) => {
      onEvent(event, true);
    },
    [onEvent],
  );
  const onTouchEnd = useCallback(
    (event: TouchEvent<HTMLDivElement>) => {
      onEvent(event, false);
    },
    [onEvent],
  );

  return (
    <ButtonPressReleaseInner
      $active={active}
      onMouseDown={onMouseDown}
      onMouseUp={onMouseUp}
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      {children}
    </ButtonPressReleaseInner>
  );
};
// TODO: support undefined
const ButtonPressReleaseInner = styled.div<{
  $active: boolean;
}>`
  ${ButtonBase}
  ${({ $active }) => ($active ? ButtonActiveMixin : null)}

  user-select: none;
  -webkit-user-select: none;
  -ms-user-select: none;
`;

export const ButtonLink: React.FC<{
  href: string;
  targetBlank?: boolean;
  children?: React.ReactNode;
}> = (props) => {
  const { href, targetBlank, children } = props;

  return (
    <ButtonLinkInner href={href} target={targetBlank ? "_blank" : undefined}>
      {children}
    </ButtonLinkInner>
  );
};
const ButtonLinkInner = styled.a`
  ${ButtonBase}
`;

export const ButtonGroup: React.FC<{
  children?: React.ReactNode;
}> = (props) => {
  return <ButtonGroupInner {...props} />;
};
const ButtonGroupInner = styled.div`
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(0, 1fr);

  & > ${ButtonActionAsyncInner}, & > ${ButtonPressReleaseInner}, & > ${ButtonLinkInner} {
    &:not(:first-child) {
      border-top-left-radius: 0;
      border-bottom-left-radius: 0;
    }

    &:not(:last-child) {
      border-top-right-radius: 0;
      border-bottom-right-radius: 0;
    }
  }
`;
