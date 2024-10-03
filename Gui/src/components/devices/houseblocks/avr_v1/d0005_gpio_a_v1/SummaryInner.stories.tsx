import { Meta } from "@storybook/react";
import Component from "./SummaryInner";

export default {
  title: "components/devices/houseblocks/avr_v1/d0005_gpio_a_v1/SummaryInner",
} satisfies Meta;

export const Basic: React.FC = () => (
  <>
    <Component
      data={{
        status_led: { r: true, g: false, b: true },
        block_1_values: [
          {
            function: "Unused",
          },
          {
            function: "AnalogIn",
            value: 5.1234,
          },
          {
            function: "DigitalIn",
            value: true,
          },
          {
            function: "DigitalIn",
            value: null,
          },
        ],
        block_2_values: [
          {
            function: "Unused",
          },
          {
            function: "Ds18x20",
            value: null,
          },
          {
            function: "Ds18x20",
            value: {
              sensor_type: "S",
              reset_count: 0,
              temperature: 265.15,
            },
          },
          {
            function: "Ds18x20",
            value: {
              sensor_type: "Invalid",
              reset_count: 3,
              temperature: null,
            },
          },
        ],
        block_3_values: [
          {
            function: "Unused",
          },
          {
            function: "AnalogIn",
            value: null,
          },
        ],
        block_4_values: [
          {
            function: "Unused",
          },
          {
            function: "DigitalOut",
            value: true,
          },
          {
            function: "DigitalOut",
            value: false,
          },
        ],
      }}
    />
  </>
);

export const Empty: React.FC = () => (
  <>
    <Component data={undefined} />
  </>
);
