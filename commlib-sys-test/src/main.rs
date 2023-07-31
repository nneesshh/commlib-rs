use commlib_sys::app_helper::App;
mod test_service;

fn main() {
  let mut app = App::new();
  app.start();
  app.attach(|| {
    test_service::G_TEST_SERVICE.clone()
  });
  app.run();
}

