import "minireset.css";
import React from "react";
import { BrowserRouter } from "react-router-dom";
import { createGlobalStyle } from "styled-components";
import "typeface-open-sans";
import Body from "./root/Body";

const App: React.FC = () => {
  return (
    <BrowserRouter>
      <GlobalStyles />
      <Body />
    </BrowserRouter>
  );
};

export default App;

const GlobalStyles = createGlobalStyle`
  html,
  body {
    font-family: "Open Sans", sans-serif;
    font-size: 11pt;
  }
`;
