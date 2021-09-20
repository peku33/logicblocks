import "minireset.css";
import { createGlobalStyle } from "styled-components";
import "typeface-open-sans";

const GlobalStyles = createGlobalStyle`
  html,
  body {
    font-family: "Open Sans", sans-serif;
    font-size: 12pt;
  }
`;
export default GlobalStyles;
