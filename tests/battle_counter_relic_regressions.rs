use sts_simulator::content::relics::{hooks, RelicId, RelicState};
use sts_simulator::test_support::blank_test_combat;

#[test]
fn captains_wheel_resets_counter_per_battle() {
    let mut combat = blank_test_combat();
    let mut relic = RelicState::new(RelicId::CaptainsWheel);
    relic.counter = 2;
    combat.entities.player.add_relic(relic);

    let _ = hooks::at_battle_start(&mut combat);

    let wheel = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::CaptainsWheel)
        .expect("Captain's Wheel should remain present after battle start");
    assert_eq!(wheel.counter, 0);

    let _ = hooks::on_victory(&mut combat);

    let wheel = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::CaptainsWheel)
        .expect("Captain's Wheel should remain present after on_victory");
    assert_eq!(wheel.counter, -1);
}

#[test]
fn stone_calendar_resets_counter_per_battle() {
    let mut combat = blank_test_combat();
    let mut relic = RelicState::new(RelicId::StoneCalendar);
    relic.counter = 6;
    combat.entities.player.add_relic(relic);

    let _ = hooks::at_battle_start(&mut combat);

    let calendar = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::StoneCalendar)
        .expect("Stone Calendar should remain present after battle start");
    assert_eq!(calendar.counter, 0);

    let _ = hooks::on_victory(&mut combat);

    let calendar = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::StoneCalendar)
        .expect("Stone Calendar should remain present after on_victory");
    assert_eq!(calendar.counter, -1);
}

#[test]
fn slavers_collar_reclaims_bonus_energy_on_victory() {
    let mut combat = blank_test_combat();
    combat.entities.player.energy_master = 4;

    let mut relic = RelicState::new(RelicId::SlaversCollar);
    relic.counter = 1;
    combat.entities.player.add_relic(relic);

    let _ = hooks::on_victory(&mut combat);

    let collar = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::SlaversCollar)
        .expect("Slaver's Collar should remain present after on_victory");
    assert_eq!(collar.counter, 0);
    assert_eq!(combat.entities.player.energy_master, 3);
}

#[test]
fn per_turn_combo_relics_clear_counters_on_victory() {
    let mut combat = blank_test_combat();
    for relic_id in [
        RelicId::Kunai,
        RelicId::Shuriken,
        RelicId::LetterOpener,
        RelicId::OrnamentalFan,
    ] {
        let mut relic = RelicState::new(relic_id);
        relic.counter = 2;
        combat.entities.player.add_relic(relic);
    }

    let _ = hooks::on_victory(&mut combat);

    for relic_id in [
        RelicId::Kunai,
        RelicId::Shuriken,
        RelicId::LetterOpener,
        RelicId::OrnamentalFan,
    ] {
        let relic = combat
            .entities
            .player
            .relics
            .iter()
            .find(|relic| relic.id == relic_id)
            .expect("combo relic should remain present after on_victory");
        assert_eq!(
            relic.counter, -1,
            "{relic_id:?} should reset its per-turn combo counter on victory"
        );
    }
}
