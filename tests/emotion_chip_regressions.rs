use sts_simulator::content::relics::{hooks, RelicId, RelicState};
use sts_simulator::test_support::blank_test_combat;

#[test]
fn emotion_chip_clears_pulse_on_victory() {
    let mut combat = blank_test_combat();
    let mut relic = RelicState::new(RelicId::EmotionChip);
    relic.counter = 1;
    combat.entities.player.add_relic(relic);

    let _ = hooks::on_victory(&mut combat);

    let chip = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::EmotionChip)
        .expect("Emotion Chip should remain present after on_victory");
    assert_eq!(
        chip.counter, 0,
        "Emotion Chip pulse should be cleared after combat so it cannot leak into the next fight"
    );
}
