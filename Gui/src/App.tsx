import GlobalStyles from "GlobalStyles";
import { BrowserRouter } from "react-router-dom";
import Body from "./root/Body";

const App: React.VFC = () => {
  return (
    <BrowserRouter>
      <GlobalStyles />
      <Body />
    </BrowserRouter>
  );
};
export default App;
