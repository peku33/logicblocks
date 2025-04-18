import GlobalStyles from "@/GlobalStyles";
import { BrowserRouter } from "react-router";
import Body from "./root/Body";

const App: React.FC = () => {
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
