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

Here is the pinout:

