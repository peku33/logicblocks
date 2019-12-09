import React from "react";
import { BrowserRouter } from "react-router-dom";
import Body from "./root/Body";

const App: React.FC = () => {
  return (
    <BrowserRouter>
      <Body />
    </BrowserRouter>
  );
};

export default App;
