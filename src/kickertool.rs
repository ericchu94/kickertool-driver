mod scraper;

use rxrust::prelude::*;

use crate::sources::browser::headless_chrome::HtmlObservable;

use self::scraper::KickertoolData;

type N = impl FnMut(KickertoolData) + Send + Sync + 'static;
type KickertoolDataObservable = impl SubscribeNext<'static, N>
    + Clone
    + Observable<Item = KickertoolData, Err = ()>
    + SharedObservable;

pub struct Kickertool {
    team_subscriptions: [Option<Box<dyn SubscriptionLike>>; 2],
    standings_subscription: Option<Box<dyn SubscriptionLike>>,
    kickertool_data_observable: KickertoolDataObservable,
}

fn get_kickertool_data_observable(
    html_observable: HtmlObservable,
) -> KickertoolDataObservable {
    html_observable
        .flat_map(|html| observable::of_option(KickertoolData::from_html(html)))
        .tap(|data| println!("Parsed data: {:?}", data))
        .distinct_until_changed()
        .tap(|data| println!("Distinct data: {:?}", data))
        .share()
        .into_shared()
}

impl Kickertool {
    pub fn new(html_observable: HtmlObservable) -> Self {
        let kickertool_data_observable = get_kickertool_data_observable(html_observable);
        let mut s = Self {
            team_subscriptions: [None, None],
            standings_subscription: None,
            kickertool_data_observable,
        };
        s.standings_subscribe();
        s.team_subscribe(1);
        s.team_subscribe(2);

        s
    }

    fn standings_subscribe(&mut self) {
        Self::unsubscribe(&mut self.standings_subscription);

        let s = self
            .kickertool_data_observable
            .clone()
            .subscribe(move |data| {
                let standings = data.standings;
                for line in &standings {
                    println!("{line}");
                }
                std::fs::write("standings.txt", standings.join("\n")).unwrap()
            });

        self.standings_subscription = (Box::new(s) as Box<dyn SubscriptionLike>).into();
    }

    fn team_subscribe(&mut self, number: usize) {
        self.team_unsubscribe(number);

        let s = self
            .kickertool_data_observable
            .clone()
            .flat_map(move |data| {
                observable::of_option(match number {
                    1 => data.team1,
                    2 => data.team2,
                    _ => unreachable!(),
                })
            })
            .distinct_until_changed()
            .into_shared()
            .subscribe(move |team| {
                println!("Team{number}: {team}");
                std::fs::write(format!("team{number}.txt"), team).unwrap();
            });

        let subscription = self.get_team_subscription_mut(number);
        *subscription = (Box::new(s) as Box<dyn SubscriptionLike>).into();
    }

    fn team_unsubscribe(&mut self, number: usize) {
        Self::unsubscribe(self.get_team_subscription_mut(number));
    }

    fn unsubscribe(subscription: &mut Option<Box<dyn SubscriptionLike>>) {
        if let Some(mut subscription) = subscription.take() {
            subscription.unsubscribe();
        }
    }

    fn get_team_subscription_mut(
        &mut self,
        number: usize,
    ) -> &mut Option<Box<dyn SubscriptionLike>> {
        &mut self.team_subscriptions[number - 1]
    }
}

impl Drop for Kickertool {
    fn drop(&mut self) {
        self.team_unsubscribe(1);
        self.team_unsubscribe(2);
        Self::unsubscribe(&mut self.standings_subscription);
    }
}
