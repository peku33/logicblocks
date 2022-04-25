import styled from "styled-components";
import Colors from "./Colors";

export const Button: React.FC<{
  active?: boolean;
  onClick?: () => void;
  onMouseDown?: () => void;
  onMouseUp?: () => void;
}> = (props) => {
  // TODO: move mouse up/down to new ButtonPress

  const { active, onClick, onMouseDown, onMouseUp, children } = props;

  return (
    <ButtonInner active={active} onClick={onClick} onMouseDown={onMouseDown} onMouseUp={onMouseUp}>
      {children}
    </ButtonInner>
  );
};
const ButtonInner = styled.div<{
  active?: boolean;
}>`
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  text-align: center;

  padding: 0.5rem 1rem;
  border-radius: 0.25rem;

  background-color: ${(props) => (props.active ? Colors.GREEN : Colors.GREY)};
  color: ${Colors.WHITE};
  font-weight: bold;
  cursor: pointer;

  :hover {
    background-color: ${(props) => (props.active ? Colors.GREEN : Colors.GREY_DARK)};
  }
`;

export const ButtonLink: React.FC<{
  href: string | undefined;
  targetBlank?: boolean;
}> = (props) => {
  const { href, targetBlank, children } = props;

  return (
    <ButtonLinkInner href={href} target={targetBlank ? "_blank" : undefined}>
      {children}
    </ButtonLinkInner>
  );
};
const ButtonLinkInner = styled.a`
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  text-align: center;

  padding: 0.5rem 1rem;
  border-radius: 0.25rem;

  background-color: ${Colors.GREY};
  color: ${Colors.WHITE};
  font-weight: bold;
  cursor: pointer;

  text-decoration: none;

  :hover {
    background-color: ${Colors.GREY_DARK};
  }
`;

export const ButtonGroup: React.FC<{}> = (props) => {
  return <ButtonGroupInner {...props} />;
};
const ButtonGroupInner = styled.div`
  display: grid;
  grid-auto-flow: column;
  /* grid-auto-columns: auto; */

  & > ${ButtonInner}, & > ${ButtonLinkInner} {
    :not(:first-child) {
      border-top-left-radius: 0;
      border-bottom-left-radius: 0;
    }

    :not(:last-child) {
      border-top-right-radius: 0;
      border-bottom-right-radius: 0;
    }
  }
`;
