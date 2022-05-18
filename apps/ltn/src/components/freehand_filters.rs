use std::collections::BTreeSet;

use geom::PolyLine;
use map_model::{IntersectionID, Perimeter};
use widgetry::tools::PolyLineLasso;
use widgetry::{DrawBaselayer, EventCtx, GfxCtx, Key, Line, ScreenPt, State, Text, Widget};

use crate::per_neighborhood::Tab;
use crate::{after_edit, App, DiagonalFilter, Neighborhood, NeighborhoodID, Transition};

pub struct FreehandFilters {
    lasso: PolyLineLasso,
    id: NeighborhoodID,
    perimeter: Perimeter,
    interior_intersections: BTreeSet<IntersectionID>,
    instructions: Text,
    instructions_at: ScreenPt,
    tab: Tab,
}

impl FreehandFilters {
    pub fn new_state(
        ctx: &EventCtx,
        neighborhood: &Neighborhood,
        instructions_at: ScreenPt,
        tab: Tab,
    ) -> Box<dyn State<App>> {
        Box::new(Self {
            lasso: PolyLineLasso::new(),
            id: neighborhood.id,
            perimeter: neighborhood.orig_perimeter.clone(),
            interior_intersections: neighborhood.interior_intersections.clone(),
            instructions_at,
            instructions: Text::from_all(vec![
                Line("Click and drag").fg(ctx.style().text_hotkey_color),
                Line(" across the roads you want to filter"),
            ]),
            tab,
        })
    }

    pub fn button(ctx: &EventCtx) -> Widget {
        ctx.style()
            .btn_outline
            .icon_text(
                "system/assets/tools/select.svg",
                "Create filters along a shape",
            )
            .hotkey(Key::F)
            .build_def(ctx)
    }

    fn make_filters_along_path(&self, ctx: &mut EventCtx, app: &mut App, path: PolyLine) {
        app.session.modal_filters.before_edit();
        for r in &self.perimeter.interior {
            if app.session.modal_filters.roads.contains_key(r) {
                continue;
            }
            let road = app.map.get_r(*r);
            if let Some((pt, _)) = road.center_pts.intersection(&path) {
                let dist = road
                    .center_pts
                    .dist_along_of_point(pt)
                    .map(|pair| pair.0)
                    .unwrap_or(road.center_pts.length() / 2.0);
                app.session.modal_filters.roads.insert(*r, dist);
            }
        }
        for i in &self.interior_intersections {
            if app.map.get_i(*i).polygon.intersects_polyline(&path) {
                // We probably won't guess the right one, but make an attempt
                DiagonalFilter::cycle_through_alternatives(ctx, app, *i);
            }
        }
        after_edit(ctx, app);
    }
}

impl State<App> for FreehandFilters {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut App) -> Transition {
        if let Some(pl) = self.lasso.event(ctx) {
            self.make_filters_along_path(ctx, app, pl);
            return Transition::Multi(vec![
                Transition::Pop,
                Transition::Replace(match self.tab {
                    Tab::Connectivity => crate::connectivity::Viewer::new_state(ctx, app, self.id),
                    // TODO Preserve the current shortcut
                    Tab::Shortcuts => {
                        crate::shortcut_viewer::BrowseShortcuts::new_state(ctx, app, self.id, None)
                    }
                }),
            ]);
        }
        Transition::Keep
    }

    fn draw(&self, g: &mut GfxCtx, _: &App) {
        self.lasso.draw(g);
        // Hacky, but just draw instructions over the other panel
        g.draw_tooltip_at(self.instructions.clone(), self.instructions_at);
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }
}