//! A menu gesture retains ownership even when its press resumes the world.
#[derive(Debug, PartialEq, Eq)]
pub enum Route {
    World,
    Menu,
    Ignore,
}
pub fn route(
    owner: &mut Option<u64>,
    id: u64,
    started: bool,
    finished: bool,
    menu_open: bool,
) -> Route {
    if let Some(active) = *owner {
        if active != id {
            return Route::Ignore;
        }
        if finished {
            *owner = None;
        }
        return Route::Menu;
    }
    if !menu_open {
        return Route::World;
    }
    if started {
        *owner = Some(id);
        Route::Menu
    } else {
        Route::Ignore
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resume_releases_its_menu_pointer_before_the_next_menu() {
        let mut owner = None;
        assert_eq!(route(&mut owner, 1, true, false, true), Route::Menu);
        // The press resumed the world; release still belongs to the menu.
        assert_eq!(route(&mut owner, 1, false, true, false), Route::Menu);
        assert_eq!(owner, None);
        assert_eq!(route(&mut owner, 2, true, false, true), Route::Menu);
    }
    #[test]
    fn secondary_finger_cannot_release_or_take_over_the_click() {
        let mut owner = Some(1);
        assert_eq!(route(&mut owner, 2, true, false, true), Route::Ignore);
        assert_eq!(route(&mut owner, 2, false, true, true), Route::Ignore);
        assert_eq!(owner, Some(1));
        assert_eq!(route(&mut owner, 1, false, true, true), Route::Menu);
        assert_eq!(route(&mut owner, 2, false, true, true), Route::Ignore);
    }
}
