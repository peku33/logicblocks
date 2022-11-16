import { makeAvrV1Summary } from "../Summary";
import { makeAvrV1SummaryManaged } from "../SummaryManaged";
import Component from "./SummaryInner";

export default makeAvrV1SummaryManaged(makeAvrV1Summary(Component));
