#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_itm as _;
//use stm32f3_discovery::{leds::Leds, stm32f3xx_hal, switch_hal::{ToggleableOutputSwitch, OutputSwitch}};
use stm32f3xx_hal as hal;
use hal::{
    prelude::*,
    gpio::{self, Output, PushPull, PXx, Input},
    pac, usb, serial, timer, Toggle,
};
use usb_device::{bus::UsbBusAllocator, class::UsbClass as _, device::UsbDeviceState};
use rtic::app;

use siberon::{keyberon, Event, active::{SiberonActive, Poll, DeBuffer}};
//type Siberon = SiberonActive<Pin<Input<PullUp>>, Pin<Output<PushPull>>>;
type Siberon = SiberonActive<PXx<Input>, PXx<Output<PushPull>>>;

type UsbClass = keyberon::Class<'static, usb::UsbBusType, ()>;
type UsbDevice = usb_device::device::UsbDevice<'static, usb::UsbBusType>;

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

#[app(device = crate::hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice,
        usb_class: UsbClass,
        siberon: Siberon,
        timer: timer::Timer<pac::TIM3>,
        tx: serial::Tx<hal::pac::USART1, gpio::PA9<gpio::AF7<PushPull>>>,
        rx: serial::Rx<hal::pac::USART1,  gpio::PA10<gpio::AF7<PushPull>>>,
    }

    #[init]
    fn init(c: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<UsbBusAllocator<usb::UsbBusType>> = None;

        let mut flash = c.device.FLASH.constrain();
        let mut rcc = c.device.RCC.constrain();
        let clocks = rcc.cfgr
            .use_hse(8.MHz())
            .sysclk(48.MHz())
            .pclk1(24.MHz())
            .pclk2(24.MHz())
            .freeze(&mut flash.acr);
    
        assert!(clocks.usbclk_valid());

        let mut gpioa = c.device.GPIOA.split(&mut rcc.ahb);
        let mut gpiob = c.device.GPIOB.split(&mut rcc.ahb);
        let mut gpiod = c.device.GPIOD.split(&mut rcc.ahb);

        {
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
                usb: c.device.USB,
                pin_dm,
                pin_dp,
            };
            *USB_BUS = Some(usb::UsbBusType::new(usb));
        }
        let usb_bus = USB_BUS.as_ref().unwrap();
        let usb_class = keyberon::new_class(usb_bus, ());
        let usb_dev = keyberon::new_device(usb_bus);

        let mut timer = timer::Timer::new(c.device.TIM3, clocks, &mut rcc.apb1);
        timer.enable_interrupt(timer::Event::Update);
        timer.start(1.milliseconds());

        let (tx, rx) = {
            let tx = gpioa.pa9.into_af7_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
            let rx = gpioa.pa10.into_af7_push_pull(&mut gpioa.moder, &mut gpioa.otyper, &mut gpioa.afrh);
            let mut serial = serial::Serial::new(c.device.USART1, (tx, rx), 57600.Bd(), clocks, &mut rcc.apb2);
            //serial.listen(serial::Event::Rxne);
            serial.enable_interrupt(serial::Event::ReceiveDataRegisterNotEmpty);
            serial.split()
        };

        let cols = input_pins!(
            gpiob => [pb10, pb12, pb14],
            gpiod => [pd8, pd10, pd12]
        );
        
        let rows = output_pins!(
            gpiod => [pd13, pd11, pd9],
            gpiob => [pb15, pb13, pb11]
        );

        let siberon = Siberon::init(cols, rows).unwrap();

        init::LateResources {
            usb_dev,
            usb_class,
            timer,
            siberon,
            tx,
            rx,
        }
    }

    #[task(binds = USART1_EXTI25, priority = 5, spawn = [handle_event], resources = [rx])]
    fn rx(c: rx::Context) {
        static mut BUF: DeBuffer = DeBuffer::new((6, 0));

        if let Ok(byte) = c.resources.rx.read() {
            if let Some(event) = BUF.feed(byte) {
                c.spawn.handle_event(event).unwrap();
            }
        }
    }

    #[task(priority = 3, capacity = 8, resources = [siberon])]
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
        c.resources.timer.wait().ok();

        let poll = c.resources.siberon.lock(|siberon| siberon.poll()).unwrap();

        c.spawn.usb_send(poll).unwrap();
    }

    extern "C" {
        fn DMA2_CH1();
        fn DMA2_CH2();
        fn DMA2_CH3();
    }
};

fn usb_poll(usb_dev: &mut UsbDevice, keyboard: &mut UsbClass) {
    if usb_dev.poll(&mut [keyboard]) {
        keyboard.poll();
    }
}
