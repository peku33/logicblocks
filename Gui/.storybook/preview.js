import styled from 'styled-components';
import GlobalStyles from '../src/GlobalStyles';

export const decorators = [
  (Story) => (
    <>
      <GlobalStyles />
      <Container>
        <Story />
      </Container>
    </>
  ),
];

const Container = styled.div`
  & > * {
    margin: 0.25rem;
  }
`;


export const parameters = {
  actions: { argTypesRegex: "^on[A-Z].*" },
  controls: {
    matchers: {
      color: /(background|color)$/i,
      date: /Date$/,
    },
  },
}
