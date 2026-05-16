# nixi-clock
My personal nixi clock with RP2040 Raspberry pico Zero


<img width="400" alt="image" src="https://github.com/user-attachments/assets/8e4ff64e-d27c-4b43-ad4d-2d11fbfacc6b" />



## Feature

- 4 Digit IN-4 Nixi Tube
- Raspberry Pico Zero W
- Ambient lighting sensor
- Temperature sensor
- DHT11 Sensor
- FM Radio with SkyLab chip Si470x

# Hardware

## Nixie Tube Driver
The nixie tubes needs high voltage to turn on the filament. Each tube have a commen node (Anode) and the chiper is turn on by a dedicate pin.
To drive the filament there is a dedicate mux, that could be driver with TTL voltage, using 4bit we can select 0-9 chiper.

<img width="600" alt="image" src="https://github.com/user-attachments/assets/828cc391-1976-4521-8c81-87580c047cb2" />

The high voltage mux driver is connected to the 2 byte SIPO:

<img width="600" alt="image" src="https://github.com/user-attachments/assets/70766219-b4f0-4f22-835c-303a1835e021" />



# Raspberry Pico WiFi Nixie Clock

| Pin # | Nome Pin (MCU) | Segnale / Net         | Resistore | Note          |
|-------|----------------|-----------------------|-----------|---------------|
| 1     | GP0            | GP0                   | R139 33R  |               |
| 2     | GP1            | GP1                   | R140 33R  |               |
| 4     | GP2            | GP2                   | R141 33R  |               |
| 5     | GP3            | GP3                   | R142 33R  |               |
| 6     | GP4            | BACK_LIGHT_DATA       | R100 33R  |               |
| 7     | GP5            | FRONT_LIGHT_DATA      | R101 33R  |               |
| 9     | GP6            | HV_ENABLE             | R114 33R  | High voltage boost Enable |
| 10    | GP7            | SIPO_RCLK             | R104 33R  | Clock Chiper SIPO|
| 11    | GP8            | SIPO_CLR              | R106 33R  | Clock Chiper SIPO|
| 12    | GP9            | SIPO_OE               | R108 33R  | Clock Chiper SIPO|
| 14    | GP10           | SIPO_DATA             | R111 33R  | Clock Chiper SIPO|
| 15    | GP11           | SIPO_CLK              | R113 33R  | Clock Chiper SIPO|
| 16    | GP12           | 1WIRE_SENSOR          | R105 33R  |               |
| 17    | GP13           | B                     | R112 33R  | External Encoder  Signal B|
| 19    | GP14           | SWITCH0               | R109 33R  | External Encoder  Button |
| 20    | GP15           | A                     | R110 33R  | External Encoder  Signal A |
| 21    | GP16           | SDIO                  | —         |               |
| 22    | GP17           | SCLK                  | —         |               |
| 24    | GP18           | RST                   | —         |               |
| 25    | GP19           | GP19                  | R144 33R  |               |
| 26    | GP20           | BUZZER                | R102 33R  |               |
| 27    | GP21           | RADIO_SW              | —         |               |
| 29    | GP22           | GP22                  | R143 33R  |               |
| 30    | RUN            | nRESET                | R103 33R  |               |
| 31    | GP26           | Raw_LOUT              | R133 33R  |               |
| 32    | GP27           | Raw_ROUT              | R132 33R  |               |
| 34    | GP28           | AMB_LIGHT             | R107 1k   | Ambient Light Sensor  |
| 35    | AVREF          | —                     | —         |               |
| 36    | 3V3            | +3.3V                 | —         | Alimentazione |
| 37    | EN             | —                     | —         | Non connesso  |
| 39    | VSYS           | —                     | —         | Non connesso  |
| 40    | VBUS           | +5V                   | —         | Alimentazione |


