//! Overlay page over the world: the pause. Picture: `ui/pause_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::pause_page::{Handler, PAGE};
use crate::world_page::WorldPage;

#[derive(Default)]
pub struct PausePage;

impl Page<AppState> for PausePage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;
    /// Root is see-through: the world stays drawn under this page.
    const KIND: PageKind = PageKind::Overlay;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            // The world's Surface still says `capture_mouse=true`: once
            // this page is gone, the world has the mouse again.
            Handler::Resume => host.nav.back(),
            Handler::StopLooking => {
                host.nav.tell::<WorldPage>(|world, host| world.set_looking(host, false));
                host.nav.back();
            }
        }
    }
}
