import { library } from "@fortawesome/fontawesome-svg-core";
import { far } from "@fortawesome/free-regular-svg-icons";
import { fas } from "@fortawesome/free-solid-svg-icons";
import "minireset.css";
import { createGlobalStyle } from "styled-components";
import "typeface-open-sans";

library.add(fas, far);

const GlobalStyles = createGlobalStyle`
  html,
  body {
    font-family: "Open Sans", sans-serif;
    font-size: 12pt;
  }

  a {
    text-decoration: None;
    color: inherit;
  }
`;
export default GlobalStyles;
