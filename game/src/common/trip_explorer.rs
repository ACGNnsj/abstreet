use crate::common::{CommonState, RoadColorer, RoadColorerBuilder};
use crate::game::{State, Transition};
use crate::helpers::{rotating_color_map, ID};
use crate::ui::UI;
use ezgui::{hotkey, EventCtx, GfxCtx, Key, Line, ModalMenu, Text, WarpingItemSlider};
use geom::Pt2D;
use map_model::PathStep;
use sim::{TripEnd, TripID, TripStart};

// TODO More info, like each leg of the trip, times, separate driving leg for looking for
// parking...
pub struct TripExplorer {
    slider: WarpingItemSlider<ID>,
}

impl TripExplorer {
    pub fn new(ctx: &mut EventCtx, ui: &UI) -> Option<TripExplorer> {
        let map = &ui.primary.map;
        let agent = ui
            .primary
            .current_selection
            .as_ref()
            .and_then(|id| id.agent_id())?;
        let trip = ui.primary.sim.agent_to_trip(agent)?;
        let status = ui.primary.sim.trip_status(trip);
        if !ctx.input.contextual_action(Key::T, "explore trip") {
            return None;
        }

        let steps: Vec<(Pt2D, ID, Text)> = vec![
            match status.start {
                TripStart::Bldg(b) => (
                    map.get_b(b).front_path.line.pt1(),
                    ID::Building(b),
                    Text::from(Line(format!("start at {}", map.get_b(b).get_name()))),
                ),
                TripStart::Border(i) => (
                    map.get_i(i).polygon.center(),
                    ID::Intersection(i),
                    Text::from(Line(format!("enter map via {}", i))),
                ),
            },
            (
                ui.primary
                    .sim
                    .get_canonical_pt_per_trip(trip, map)
                    .ok()
                    .unwrap(),
                ID::from_agent(agent),
                Text::from(Line("currently here")),
            ),
            match status.end {
                TripEnd::Bldg(b) => (
                    map.get_b(b).front_path.line.pt1(),
                    ID::Building(b),
                    Text::from(Line(format!("end at {}", map.get_b(b).get_name()))),
                ),
                TripEnd::Border(i) => (
                    map.get_i(i).polygon.center(),
                    ID::Intersection(i),
                    Text::from(Line(format!("leave map via {}", i))),
                ),
                TripEnd::ServeBusRoute(br) => {
                    let route = map.get_br(br);
                    let stop = map.get_bs(route.stops[0]);
                    (
                        stop.driving_pos.pt(map),
                        ID::BusStop(stop.id),
                        Text::from(Line(format!("serve route {} forever", route.name))),
                    )
                }
            },
        ];

        Some(TripExplorer {
            slider: WarpingItemSlider::new(
                steps,
                &format!("Trip Explorer for {}", trip),
                "step",
                ctx,
            ),
        })
    }
}

impl State for TripExplorer {
    fn event(&mut self, ctx: &mut EventCtx, ui: &mut UI) -> Transition {
        if ctx.redo_mouseover() {
            ui.recalculate_current_selection(ctx);
        }
        ctx.canvas.handle_event(ctx.input);

        if let Some((evmode, done_warping)) = self.slider.event(ctx) {
            if done_warping {
                ui.primary.current_selection = Some(self.slider.get().1.clone());
            }
            Transition::KeepWithMode(evmode)
        } else {
            Transition::Pop
        }
    }

    fn draw(&self, g: &mut GfxCtx, ui: &UI) {
        self.slider.draw(g);
        CommonState::draw_osd(g, ui, &ui.primary.current_selection);
    }
}

pub struct NewTripExplorer {
    menu: ModalMenu,
    // TODO Or path traces?
    colorer: RoadColorer,
}

impl NewTripExplorer {
    pub fn new(trip: TripID, ctx: &mut EventCtx, ui: &UI) -> NewTripExplorer {
        let phases = ui
            .primary
            .sim
            .get_analytics()
            .get_trip_phases(trip, &ui.primary.map);
        // TODO Hack because ColorLegend only takes &str
        let mut rows = Vec::new();
        for (idx, p) in phases.iter().enumerate() {
            let label = if let Some(t2) = p.end_time {
                format!("{} .. {} ({})", p.start_time, t2, t2 - p.start_time)
            } else {
                format!(
                    "{} .. ongoing ({} so far)",
                    p.start_time,
                    ui.primary.sim.time() - p.start_time
                )
            };
            rows.push((
                format!("{}: {}", label, p.description),
                rotating_color_map(idx),
            ));
        }
        let mut colorer = RoadColorerBuilder::new(
            Text::prompt(&trip.to_string()),
            rows.iter()
                .map(|(label, color)| (label.as_str(), *color))
                .collect(),
        );
        for (p, (_, color)) in phases.iter().zip(rows.iter()) {
            if let Some(ref path) = p.path {
                for s in path.get_steps() {
                    match s {
                        PathStep::Lane(l) | PathStep::ContraflowLane(l) => {
                            colorer.add(*l, *color, &ui.primary.map);
                        }
                        _ => {}
                    }
                }
            }
        }

        NewTripExplorer {
            menu: ModalMenu::new(trip.to_string(), vec![(hotkey(Key::Escape), "quit")], ctx),
            colorer: colorer.build(ctx, &ui.primary.map),
        }
    }
}

impl State for NewTripExplorer {
    fn event(&mut self, ctx: &mut EventCtx, ui: &mut UI) -> Transition {
        if ctx.redo_mouseover() {
            ui.recalculate_current_selection(ctx);
        }
        ctx.canvas.handle_event(ctx.input);

        self.menu.event(ctx);
        if self.menu.action("quit") {
            return Transition::Pop;
        }

        Transition::Keep
    }

    fn draw_default_ui(&self) -> bool {
        false
    }

    fn draw(&self, g: &mut GfxCtx, ui: &UI) {
        self.colorer.draw(g, ui);
        self.menu.draw(g);
    }
}
