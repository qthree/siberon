#![deny(unsafe_code)]
#![no_main]
#![no_std]

mod layout;

#[allow(unused)]
use panic_itm;
use cortex_m::iprintln;
use cortex_m_rt::entry;
use stm32f3_discovery::{leds::Leds, stm32f3xx_hal, switch_hal::{ToggleableOutputSwitch, OutputSwitch}};
use stm32f3xx_hal::prelude::*;
pub use stm32f3xx_hal::{
    delay::Delay,
    gpio::{gpioe, Output, PushPull},
    hal::blocking::delay::DelayMs,
    pac, usb,
};

use heapless::String;

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
    let mut reset_and_clock_control = device_periphs.RCC.constrain();

    let core_periphs = cortex_m::Peripherals::take().unwrap();
    let mut flash = device_periphs.FLASH.constrain();
    //let clocks = reset_and_clock_control.cfgr.freeze(&mut flash.acr);
    let clocks = reset_and_clock_control.cfgr
        .use_hse(8.MHz())
        .sysclk(48.MHz())
        .pclk1(24.MHz())
        .pclk2(24.MHz())
        .freeze(&mut flash.acr);
    let mut delay = Delay::new(core_periphs.SYST, clocks);

    let mut itm = core_periphs.ITM;

    // initialize user leds
    let mut gpioa = device_periphs.GPIOA.split(&mut reset_and_clock_control.ahb);
    let mut gpiob = device_periphs.GPIOB.split(&mut reset_and_clock_control.ahb);
    let mut gpiod = device_periphs.GPIOD.split(&mut reset_and_clock_control.ahb);
    let mut gpioe = device_periphs.GPIOE.split(&mut reset_and_clock_control.ahb);
    
    let mut leds = Leds::new(
        gpioe.pe8,
        gpioe.pe9,
        gpioe.pe10,
        gpioe.pe11,
        gpioe.pe12,
        gpioe.pe13,
        gpioe.pe14,
        gpioe.pe15,
        &mut gpioe.moder,
        &mut gpioe.otyper,
    ).into_array();

    let cols = input_pins!(
        gpiob => [pb10, pb12, pb14],
        gpiod => [pd8, pd10, pd12]
    );
    
    let rows = output_pins!(
        gpiod => [pd13, pd11, pd9],
        gpiob => [pb15, pb13, pb11]
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
    

    /*let input = gpioc.pc6.into_pull_up_input(&mut gpioc.moder, &mut gpioc.pupdr);
    let mut output = gpioc.pc8.into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper);
    output.set_low().unwrap();*/
    
    //let input = gpioc.pc8.into_pull_up_input(&mut gpioc.moder, &mut gpioc.pupdr);
    //let mut output = gpioc.pc6.into_push_pull_output(&mut gpioc.moder, &mut gpioc.otyper);
    //output.set_low().unwrap();

    //let mut buffer: String<255> = String::new();

    let mut blink = 0;
    loop {
        if blink == 0{
            leds[0].toggle().unwrap();
            blink = 1000;
        } else {
            blink -= 1;
        }

        //let pressed_keys = matrix.get().unwrap();
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
        
        /*buffer.clear();
        for (row_i, row) in pressed_keys.0.iter().enumerate() {
            for (col_i, button) in row.iter().enumerate() {
                buffer.push_str(" [").unwrap();
                if *button {
                    buffer.push((b'A' + col_i as u8) as char).unwrap();
                    buffer.push((b'1' + row_i as u8) as char).unwrap();
                } else {
                    buffer.push_str("  ").unwrap();
                }
                buffer.push_str("] ").unwrap();
            }    
            buffer.push('\n').unwrap();
        }
        iprintln!(&mut itm.stim[0], "\n\n\n\n{}", buffer);
        */

        /*if input.is_low().unwrap() {
            leds[1].on().unwrap();
        } else {
            leds[1].off().unwrap();
        }*/
        //delay.delay_ms(200u16);
    }
}
