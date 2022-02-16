use log::info;
use smithay_client_toolkit::{
    default_environment,
    environment::SimpleGlobal,
    new_default_environment,
    reexports::{
        calloop::{EventLoop, LoopHandle},
        protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1,
    },
    WaylandSource,
};

use std::{
    cell::{Cell, RefCell},
    time::Duration,
};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{cli, command, renderer::Renderer};

#[derive(PartialEq, Debug)]
pub enum LoopAction {
    Redraw,
}

#[derive(Clone)]
pub struct LoopContext {
    pub action: Rc<Cell<Option<LoopAction>>>,
    pub app_context: Rc<RefCell<AppContext>>,
    pub handle: LoopHandle<'static, LoopContext>,
}

pub struct Filter(pub String);

pub struct AppContext {
    pub input: Filter,
    pub list: command::CommandList,
}

pub struct App {
    args: cli::Args,
    event_loop: EventLoop<'static, LoopContext>,
}

impl Deref for Filter {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Filter {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

impl LoopContext {
    fn new(handle: LoopHandle<'static, LoopContext>, app_context: AppContext) -> Self {
        Self {
            action: Rc::new(Cell::new(None)),
            app_context: Rc::new(RefCell::new(app_context)),
            handle,
        }
    }
}

impl AppContext {
    pub fn target(&self) -> Option<&command::Command> {
        self.list.filtered.first()
    }
    pub fn filter(&mut self) {
        self.list.filter(&self.input);
        info!("{}", self.list);
    }
}

default_environment!(RMenuEnv,
  fields = [
      layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
  ],
  singles = [
      zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell
  ],
);

impl App {
    pub fn new(args: cli::Args) -> std::io::Result<Self> {
        let event_loop = EventLoop::<LoopContext>::try_new()?;

        Ok(App {
            args,
            event_loop
        })
    }
    pub fn run(&mut self) -> std::io::Result<()> {
        let (env, display, queue) =
            new_default_environment!(RMenuEnv, fields = [layer_shell: SimpleGlobal::new(),])
                .expect("Initial roundtrip failed!");

        let app_context = AppContext {
            input: Filter(String::new()),
            list: command::CommandList::new()?,
        };

        info!("{}", app_context.list);

        WaylandSource::new(queue).quick_insert(self.event_loop.handle())?;

        // create our loop context
        let mut loop_context = LoopContext::new(self.event_loop.handle(), app_context);

        let renderer = Renderer::new(env, loop_context.clone());

        loop {
            renderer.handle_events(loop_context.action.take() == Some(LoopAction::Redraw));

            display.flush().unwrap();
            self.event_loop
                .dispatch(Duration::from_millis(100), &mut loop_context)?;
        }
    }
}
