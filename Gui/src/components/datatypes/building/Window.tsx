import { Chip, ChipType, ChipsGroup } from "@/components/common/Chips";
import { WindowOpenStateOpenClosed, WindowOpenStateOpenTiltedClosed } from "@/datatypes/building/Window";

export const WindowOpenStateOpenClosedComponent: React.FC<{
  value: WindowOpenStateOpenClosed | null | undefined;
}> = (props) => {
  const { value } = props;

  return (
    <ChipsGroup>
      <Chip type={ChipType.OK} enabled={value !== null && value === WindowOpenStateOpenClosed.Closed}>
        Closed
      </Chip>
      <Chip type={ChipType.WARNING} enabled={value !== null && value === WindowOpenStateOpenClosed.Open}>
        Open
      </Chip>
    </ChipsGroup>
  );
};

export const WindowOpenStateOpenTiltedClosedComponent: React.FC<{
  value: WindowOpenStateOpenTiltedClosed | null | undefined;
}> = (props) => {
  const { value } = props;

  return (
    <ChipsGroup>
      <Chip type={ChipType.OK} enabled={value !== null && value === WindowOpenStateOpenTiltedClosed.Closed}>
        Closed
      </Chip>
      <Chip type={ChipType.INFO} enabled={value !== null && value === WindowOpenStateOpenTiltedClosed.Tilted}>
        Tilted
      </Chip>
      <Chip type={ChipType.WARNING} enabled={value !== null && value === WindowOpenStateOpenTiltedClosed.Open}>
        Open
      </Chip>
    </ChipsGroup>
  );
};
