#![deny(unsafe_code)]
#![no_main]
#![no_std]

#[allow(unused)]
use panic_itm;
use cortex_m::iprintln;
use cortex_m_rt::entry;
//use stm32f3_discovery::{leds::Leds, stm32f3xx_hal, switch_hal::{ToggleableOutputSwitch, OutputSwitch}};
use stm32f3xx_hal as hal;
use hal::{
    prelude::*,
    delay::Delay,
    gpio::{Output, PushPull},
    pac, usb,
};
use embedded_hal::{blocking::delay::DelayMs, digital::v2::{IoPin, PinState}};

use heapless::String;

use siberon::{SiberonActive, keyberon};

macro_rules! input_pins {
    ($($gpio:expr => [$($pin:ident),*]),*) => {
        [$(
            $(
                $gpio.$pin.into_pull_up_input(&mut $gpio.moder, &mut $gpio.pupdr).downgrade().downgrade(),
            )*
        )*]
    }
}
macro_rules! output_pins {
    ($($gpio:expr => [$($pin:ident),*]),*) => {
        [$(
            $(
                $gpio.$pin.into_push_pull_output(&mut $gpio.moder, &mut $gpio.otyper).downgrade().downgrade(),
            )*
        )*]
    }
}

#[entry]
fn main() -> ! {
    let device_periphs = pac::Peripherals::take().unwrap();
    let mut rcc = device_periphs.RCC.constrain();

    let core_periphs = cortex_m::Peripherals::take().unwrap();
    let mut flash = device_periphs.FLASH.constrain();

    let clocks = rcc.cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .pclk1(24.MHz())
        .pclk2(24.MHz())
        .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut delay = Delay::new(core_periphs.SYST, clocks);

    let mut gpioa = device_periphs.GPIOA.split(&mut rcc.ahb);
    let mut gpiob = device_periphs.GPIOB.split(&mut rcc.ahb);
    let mut gpiod = device_periphs.GPIOD.split(&mut rcc.ahb);
    let mut gpioe = device_periphs.GPIOE.split(&mut rcc.ahb);

    let mut led = gpioe.pe8.into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper);
   
    
    let cols = input_pins!(
        gpiob => [pb10, pb12, pb14],
        gpiod => [pd8, pd10, pd12]
    );
    
    let rows = output_pins!(
        gpiod => [pd13, pd11, pd9],
        gpiob => [pb15, pb13, pb11]
    );

    let mut siberon = SiberonActive::init(cols, rows).unwrap();

    // F3 Discovery board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    // This forced reset is needed only for development, without it host
    // will not reset your device when you upload new firmware.
    let mut usb_dp = gpioa
        .pa12
        .into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);
    usb_dp.set_low().ok();
    cortex_m::asm::delay (clocks.sysclk().0 / 100);

    let pin_dm =
        gpioa
            .pa11
            .into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
    let pin_dp = usb_dp.into_af14_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);

    let usb = usb::Peripheral {
        usb: device_periphs.USB,
        pin_dm,
        pin_dp,
    };

    let usb_bus = usb::UsbBusType::new(usb);
    let mut usb_class = keyberon::new_class(&usb_bus, ());
    let mut usb_dev = keyberon::new_device(&usb_bus);

    let mut blink = 0;
    loop {
        if blink == 0{
            let _ = led.toggle();
            blink = 1000;
        } else {
            blink -= 1;
        }

        siberon.poll();

        if usb_dev.poll(&mut [&mut usb_class]) {
            use usb_device::class::UsbClass as _;
            usb_class.poll();
        }

        if blink % 10 == 0 {
            use usb_device::device::UsbDeviceState;

            let _tick = layout.tick();
            if usb_dev.state() == UsbDeviceState::Configured {
                /*match tick {
                    CustomEvent::Release(()) => unsafe { cortex_m::asm::bootload(0x1FFFC800 as _) },
                    _ => (),
                }*/
                let report = siberon.report();
                if usb_class.device_mut().set_keyboard_report(report.clone())
                {
                    while let Ok(0) = usb_class.write(report.as_bytes()) {
                        if usb_dev.poll(&mut [&mut usb_class]) {
                            use usb_device::class::UsbClass as _;
                            usb_class.poll();
                        }
                    }
                }
            }
        }
    }
}
