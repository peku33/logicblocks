import styled from "styled-components";
import Colors from "./Colors";
import MediaQueries from "./MediaQueries";

export const Button = styled.div<{
  active?: boolean;
}>`
  padding: 0.5rem 1rem;
  background-color: ${(props): string => (props.active ? Colors.GREEN : Colors.GREY)};
  color: white;
  font-weight: bold;
  text-align: center;
  cursor: pointer;

  @media ${MediaQueries.COMPUTER_AT_LEAST} {
    padding: 1rem 2rem;
  }

  :hover {
    background-color: ${(props): string => (props.active ? Colors.GREEN : Colors.GREY_DARK)};
  }
`;
export const ButtonGroup = styled.div`
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: max-content;

  > ${Button} {
    :first-child {
      border-radius: 0.25rem 0 0 0.25rem;
    }

    :last-child {
      border-radius: 0 0.25rem 0.25rem 0;
    }
  }
`;
