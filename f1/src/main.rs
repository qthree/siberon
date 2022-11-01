#![deny(unsafe_code)]
#![no_main]
#![no_std]

//use cortex_m::iprintln;
//use panic_itm as _;
use panic_halt as _;
//use stm32f3_discovery::{leds::Leds, stm32f3xx_hal, switch_hal::{ToggleableOutputSwitch, OutputSwitch}};
use stm32f1xx_hal as hal;
use hal::{
    prelude::*,
    gpio::{Input, Output, PushPull, PullUp, ErasedPin},
    pac, usb, timer, serial,
};
use usb_device::{bus::UsbBusAllocator, class::UsbClass as _, device::UsbDeviceState};
use rtic::app;

use siberon::{keyberon, Event, active::{SiberonActive, Poll, DeBuffer}};
type Siberon = SiberonActive<ErasedPin<Input<PullUp>>, ErasedPin<Output<PushPull>>>;

type UsbClass = keyberon::Class<'static, usb::UsbBusType, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, usb::UsbBusType>;

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


#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        siberon: Siberon,
        timer: timer::CountDownTimer<pac::TIM3>,
        tx: serial::Tx<hal::pac::USART1>,
        rx: serial::Rx<hal::pac::USART1>,
    }

    #[init]
    fn init(c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        let mut flash = c.device.FLASH.constrain();
        let rcc = c.device.RCC.constrain();
        let clocks = rcc.cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);
    
        assert!(clocks.usbclk_valid());

        let mut afio = c.device.AFIO.constrain();

        let mut gpioa = c.device.GPIOA.split();
        let mut gpiob = c.device.GPIOB.split();

        {
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
                usb: c.device.USB,
                pin_dm,
                pin_dp,
            };
            *USB_BUS = Some(usb::UsbBusType::new(usb));
        }
        let usb_bus = USB_BUS.as_ref().unwrap();
        let usb_class = keyberon::new_class(usb_bus, ());
        let usb_dev = keyberon::new_device(usb_bus);

        let mut timer = timer::Timer::tim3(c.device.TIM3, &clocks).start_count_down(1.khz());
        timer.listen(timer::Event::Update);

        let (tx, rx) = {
            let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
            let rx = gpioa.pa10;
            let mut serial = serial::Serial::usart1(c.device.USART1, (tx, rx), &mut afio.mapr, serial::Config::default().baudrate(57600.bps()), clocks);
            serial.listen(serial::Event::Rxne);
            serial.split()
        };

        let cols = input_pins!(
            gpioa; crl => [pa0, pa1, pa2, pa3, pa4, pa5]
        );
        
        let rows = output_pins!(
            gpioa; crl => [pa6, pa7],
            gpiob; crl => [pb0, pb1],
            gpiob; crh => [pb10, pb11]
        );

        let siberon = Siberon::init(cols, rows).unwrap();

        //let mut itm = c.core.ITM;
        //iprintln!(&mut itm.stim[0], "Hello world!");

        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            siberon,
            tx,
            rx,
        }
    }

    #[task(binds = USART1, priority = 5, spawn = [handle_event], resources = [rx])]
    fn rx(c: rx::Context) {
        static mut BUF: DeBuffer = DeBuffer::new((6, 0));

        if let Ok(byte) = c.resources.rx.read() {
            if let Some(event) = BUF.feed(byte) {
                let _ = c.spawn.handle_event(event);
            }
        }
    }

    #[task(priority = 2, capacity = 8, resources = [siberon])]
    fn handle_event(c: handle_event::Context, event: Event) {
        c.resources.siberon.handle_event(event);
    }

    #[task(binds = USB_HP_CAN_TX, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_tx(mut c: usb_tx::Context) {
        usb_poll(&mut c.resources.usb_dev, &mut c.resources.usb_class);
    }

    #[task(binds = USB_LP_CAN_RX0, priority = 4, resources = [usb_dev, usb_class])]
    fn usb_rx(mut c: usb_rx::Context) {
        usb_poll(&mut c.resources.usb_dev, &mut c.resources.usb_class);
    }

    #[task(priority = 2, capacity = 8, resources = [usb_dev, usb_class])]
    fn usb_send(mut c: usb_send::Context, poll: Poll) {
        let Poll{custom_event, report} = poll;
        match custom_event {
            //CustomEvent::Release(()) => unsafe { cortex_m::asm::bootload(0x1FFFC800 as _) },
            _ => (),
        }

        if c.resources.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        
        if !c
            .resources
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        while let Ok(0) = c.resources.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(
        binds = TIM3,
        priority = 1,
        spawn = [usb_send],
        resources = [siberon, timer],
    )]
    fn tick(mut c: tick::Context) {
        //c.resources.timer.wait().ok();

        let poll = c.resources.siberon.lock(|siberon| siberon.poll()).unwrap();
        //iprintln!(&mut c.resources.itm.stim[0], "{:?}", poll);

        // ignore Err if usb is disconnected
        let _ = c.spawn.usb_send(poll);

        c.resources.timer.clear_update_interrupt_flag();
    }

    extern "C" {
        fn CAN_RX1();
    }
};

fn usb_poll(usb_dev: &mut UsbDevice, keyboard: &mut UsbClass) {
    if usb_dev.poll(&mut [keyboard]) {
        keyboard.poll();
    }
}


/*
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

    let mut siberon = Siberon::init(cols, rows).unwrap();;

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

        if usb_dev.poll(&mut [&mut usb_class]) {
            use usb_device::class::UsbClass as _;
            usb_class.poll();
        }

        if blink % 10 == 0 {
            let poll = siberon.poll().unwrap();
            /*match poll.custom_event {
                CustomEvent::Release(()) => unsafe { cortex_m::asm::bootload(0x1FFFC800 as _) },
                _ => (),
            }*/

            use usb_device::device::UsbDeviceState;

            if usb_dev.state() == UsbDeviceState::Configured {
                if usb_class.device_mut().set_keyboard_report(poll.report.clone())
                {
                    while let Ok(0) = usb_class.write(poll.report.as_bytes()) {
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
*/