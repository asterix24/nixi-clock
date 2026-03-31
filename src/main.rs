#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]

use core::str::from_utf8;

use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::{bind_interrupts, dma};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embedded_io_async::Write;
use static_cell::StaticCell;

use heapless::{String, Vec};

type NetStack = embassy_net::Stack<'static>;
type NetControl = cyw43::Control<'static>;

// defmt Logging
use defmt::*;
use {defmt_rtt as _, panic_probe as _};

use nixi_clock::proto_parser::{ParserMgr, reply_err, reply_ok};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

const WIFI_NETWORK: &str = env!("SSID");
const WIFI_PASSWORD: &str = env!("PASSWORD");

static PROTO_PARSE: Channel<CriticalSectionRawMutex, String<128>, 2> = Channel::new();
static PROTO_RET: Channel<CriticalSectionRawMutex, String<64>, 2> = Channel::new();

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
            control.gpio_set(1, false).await;
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
            println!("accept error: {:?}", e);
            continue;
        }
        loop {
            let n = match socket.read(&mut tmp_buffer).await {
                Ok(0) => {
                    println!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    println!("read error: {:?}", e);
                    break;
                }
            };

            println!("rxd {}", from_utf8(&tmp_buffer[..n]).unwrap());

            PROTO_PARSE
                .send(String::from_utf8(Vec::from_slice(&tmp_buffer[..n]).unwrap()).unwrap())
                .await;

            let ret_str = PROTO_RET.receive().await;
            if let Err(e) = socket.write_all(ret_str.as_bytes()).await {
                println!("write error: {:?}", e);
                break;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let fw = aligned_bytes!("../firmware/43439A0.bin");
    let clm = aligned_bytes!("../firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../firmware/nvram_rp2040.bin");

    let mut led = Output::new(p.PIN_16, Level::Low);
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

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;

    spawner.spawn(unwrap!(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    static STACK: StaticCell<NetStack> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    let stack = STACK.init(stack);

    spawner.spawn(unwrap!(net_task(runner)));
    spawner.spawn(unwrap!(connection_task(stack, control)));
    spawner.spawn(unwrap!(shell_task(stack)));

    let in_chan = PROTO_PARSE.dyn_receiver();
    let out_chan = PROTO_RET.dyn_sender();

    loop {
        let pkg = ParserMgr::new(in_chan.receive().await);
        let reply = match pkg.cmd.as_str() {
            "led" => {
                led.set_low();
                Ok("")
            }
            _ => Err("Invalid Command"),
        };

        let ret = match reply {
            Ok(e) => reply_ok(e),
            Err(e) => reply_err(e),
        };
        out_chan.send(ret).await;
    }
}
