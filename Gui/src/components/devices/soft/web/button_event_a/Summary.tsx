import styled from "styled-components";

const Summary: React.VFC<{
  onSignal: () => void | undefined;
}> = (props) => {
  const { onSignal } = props;

  return <Button onClick={onSignal !== undefined ? onSignal : () => ({})}>Signal</Button>;
};
export default Summary;

const Button = styled.div``;
