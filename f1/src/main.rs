#![deny(unsafe_code)]
#![no_main]
#![no_std]

#[allow(unused)]
use panic_itm;
use cortex_m::iprintln;
use cortex_m_rt::entry;
//use stm32f3_discovery::{leds::Leds, stm32f3xx_hal, switch_hal::{ToggleableOutputSwitch, OutputSwitch}};
use stm32f1xx_hal as hal;
use hal::{
    prelude::*,
    delay::Delay,
    gpio::{Output, PushPull},
    pac, usb,
};
use embedded_hal::{blocking::delay::DelayMs, digital::v2::{IoPin, PinState}};

use heapless::String;

use siberon::{layout, keyberon};

macro_rules! input_pins {
    ($($gpio:expr; $cr:ident => [$($pin:ident),*]),*) => {
        [$(
            $(
                $gpio.$pin.into_pull_up_input(&mut $gpio.$cr).erase(),
            )*
        )*]
    }
}
macro_rules! output_pins {
    ($($gpio:expr; $cr:ident => [$($pin:ident),*]),*) => {
        [$(
            $(
                $gpio.$pin.into_push_pull_output(&mut $gpio.$cr).erase(),
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
        .use_hse(8.mhz())
        .sysclk(48.mhz())
        .pclk1(24.mhz())
        .freeze(&mut flash.acr);

    assert!(clocks.usbclk_valid());

    let mut delay = Delay::new(core_periphs.SYST, clocks);

    let mut gpioa = device_periphs.GPIOA.split();
    let mut gpiob = device_periphs.GPIOB.split();
    let pb13 = gpiob.pb3;
    let mut gpioc = device_periphs.GPIOC.split();

    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    
    let cols = input_pins!(
        gpioa; crl => [pa0, pa1, pa2, pa3, pa4, pa5]
    );
    
    let rows = output_pins!(
        gpioa; crl => [pa6, pa7],
        gpiob; crl => [pb0, pb1],
        gpiob; crh => [pb10, pb11]
    );

    use keyberon::{
        matrix::Matrix,
        debounce::Debouncer,
        matrix::PressedKeys,
        layout::{Layout, CustomEvent},
        key_code::KbHidReport,
    };
    let mut matrix = Matrix::new(cols, rows).unwrap();
    let mut debouncer = Debouncer::new(PressedKeys::<6, 6>::default(), PressedKeys::default(), 5);
    let mut layout = Layout::new(crate::layout::LAYERS);

    // BluePill board has a pull-up resistor on the D+ line.
    // Pull the D+ pin down to send a RESET condition to the USB bus.
    // This forced reset is needed only for development, without it host
    // will not reset your device when you upload new firmware.
    let pin_dp = {
        
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        usb_dp.set_low();
        cortex_m::asm::delay(clocks.sysclk().0 / 100);
        usb_dp.into_floating_input(&mut gpioa.crh)
    };
    let pin_dm = gpioa.pa11;

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

        for event in debouncer
            .events(matrix.get().unwrap())
        {
            layout.event(event);
        }

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
                let report: KbHidReport = layout.keycodes().collect();
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
