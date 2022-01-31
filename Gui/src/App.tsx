import GlobalStyles from "GlobalStyles";
import { BrowserRouter } from "react-router-dom";
import Body from "./root/Body";

const App: React.VFC = () => {
  return (
    <>
      <GlobalStyles />
      <BrowserRouter>
        <Body />
      </BrowserRouter>
    </>
  );
};
export default App;
