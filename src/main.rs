#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use core::str::from_utf8;

use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

use embassy_executor::Spawner;
use embassy_net::StackResources;
use embassy_net::tcp::TcpSocket;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0, SPI0, USB};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::spi::{Blocking, Phase, Polarity, Spi};
use embassy_rp::{bind_interrupts, dma, usb};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embedded_io_async::Write;
use static_cell::StaticCell;

use heapless::{String, Vec};
use log::{error, info};

use defmt::*;
// defmt Logging
use {defmt_rtt as _, panic_probe as _};

use nixi_clock::parser::{Command, reply};

type NetStack = embassy_net::Stack<'static>;
type NetControl = cyw43::Control<'static>;

const HELLO_MSG: &str = concat!(
    "\n== Hello, Nixi Clock v",
    env!("CARGO_PKG_VERSION"),
    " here! ==\n"
);

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const WIFI_NETWORK: &str = env!("SSID");
const WIFI_PASSWORD: &str = env!("PASSWORD");

static PROTO_PARSE: Channel<CriticalSectionRawMutex, String<128>, 2> = Channel::new();
static PROTO_RET: Channel<CriticalSectionRawMutex, String<64>, 2> = Channel::new();
static SIPO_OUT: Channel<CriticalSectionRawMutex, u8, 2> = Channel::new();

#[embassy_executor::task]
async fn sipo_task(mut sipo: Spi<'static, SPI0, Blocking>, mut latch: Output<'static>) {
    loop {
        let data = SIPO_OUT.receive().await;
        if let Ok(_) = sipo.blocking_write(&mut [data]) {
            embassy_time::Timer::after_micros(1).await;
            latch.set_level(Level::High);
            embassy_time::Timer::after_micros(1).await;
            latch.set_level(Level::Low);
        } else {
            error!("unable to write sipo");
        }

        embassy_time::Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    let driver = embassy_rp::usb::Driver::new(usb, Irqs);
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn clock_task() -> ! {
    let mut data: u8 = 0;
    let sipo_out = SIPO_OUT.dyn_sender();
    loop {
        embassy_time::Timer::after_millis(200).await;
        sipo_out.send(data).await;
        if data == 0 {
            data = 1;
        }
        data = data << 1;
    }
}

#[embassy_executor::task]
async fn connection_task(stack: &'static NetStack, mut control: NetControl) -> ! {
    loop {
        if !stack.is_link_up() {
            while let Err(err) = control
                .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
                .await
            {
                info!("join failed: {:?}", err);
            }
            info!("waiting for link...");
            stack.wait_link_up().await;

            info!("waiting for DHCP...");
            stack.wait_config_up().await;

            // And now we can use it!
            info!("Stack is up!");
            if let Some(config) = stack.config_v4() {
                info!("IP: {}", config.address);
            }
            control.gpio_set(0, false).await;
        }
        embassy_time::Timer::after_secs(5).await;
    }
}

#[embassy_executor::task]
async fn shell_task(stack: &'static NetStack) -> ! {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut tmp_buffer = [0; 4096];

    loop {
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(embassy_time::Duration::from_secs(60)));
        if let Err(e) = socket.accept(20000).await {
            info!("accept error: {:?}", e);
            continue;
        }
        if let Err(e) = socket.write_all(HELLO_MSG.as_bytes()).await {
            info!("write error: {:?}", e);
        }
        loop {
            let n = match socket.read(&mut tmp_buffer).await {
                Ok(0) => {
                    info!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    info!("read error: {:?}", e);
                    break;
                }
            };

            info!("rxd {}", from_utf8(&tmp_buffer[..n]).unwrap());

            PROTO_PARSE
                .send(String::from_utf8(Vec::from_slice(&tmp_buffer[..n]).unwrap()).unwrap())
                .await;

            let ret_str = PROTO_RET.receive().await;
            if let Err(e) = socket.write_all(ret_str.as_bytes()).await {
                info!("write error: {:?}", e);
                break;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    spawner.spawn(unwrap!(logger_task(p.USB)));

    let mut rng = RoscRng;

    let fw = aligned_bytes!("../firmware/43439A0.bin");
    let clm = aligned_bytes!("../firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../firmware/nvram_rp2040.bin");

    let mut led = Output::new(p.PIN_22, Level::Low);
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );

    // Sipo and Piso constants
    //sipoEnable machine.Pin = machine.GPIO15
    //sipoLoad               = machine.GPIO17
    //sipoClk                = machine.GPIO18
    //sipoMosi               = machine.GPIO19
    //pisoMiso               = machine.GPIO16
    //pisoLoad               = machine.GPIO14
    let sipo_load = Output::new(p.PIN_17, Level::Low);
    let mut sipo_en = Output::new(p.PIN_15, Level::High);
    let mut sipo_config = embassy_rp::spi::Config::default();
    sipo_config.frequency = 1_000_000;
    sipo_config.phase = Phase::CaptureOnFirstTransition;
    sipo_config.polarity = Polarity::IdleLow;
    let sipo = Spi::new_blocking_txonly(p.SPI0, p.PIN_18, p.PIN_19, sipo_config);
    sipo_en.set_level(Level::Low);

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;

    spawner.spawn(unwrap!(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = embassy_net::Config::dhcpv4(Default::default());

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    static STACK: StaticCell<NetStack> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        rng.next_u64(),
    );

    let stack = STACK.init(stack);

    spawner.spawn(unwrap!(net_task(runner)));
    spawner.spawn(unwrap!(connection_task(stack, control)));
    spawner.spawn(unwrap!(shell_task(stack)));
    spawner.spawn(unwrap!(sipo_task(sipo, sipo_load)));
    spawner.spawn(unwrap!(clock_task()));

    let in_chan = PROTO_PARSE.dyn_receiver();
    let out_chan = PROTO_RET.dyn_sender();
    let sipo_out = SIPO_OUT.dyn_sender();

    loop {
        let return_msg = match Command::try_from(in_chan.receive().await) {
            Ok(cmd) => match cmd {
                Command::Version => reply(env!("CARGO_PKG_VERSION"), true),
                Command::Out { mask } => {
                    sipo_out.send(mask).await;
                    reply("", true)
                }
                _ => reply("Invalid Command", true),
            },
            Err(_) => reply("Uknow Connand: {}", true),
        };
        out_chan.send(return_msg).await;
        led.toggle();
    }
}
