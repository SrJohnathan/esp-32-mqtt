
use dht_sensor::dht22;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripherals::Peripherals,
    nvs::EspDefaultNvsPartition,
    wifi::{ClientConfiguration, Configuration, EspWifi},
};
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::{delay, gpio};
use esp_idf_svc::hal::gpio::{Gpio2, Output, PinDriver};

use esp_idf_svc::mqtt::client::{EspMqttClient, EventPayload, MqttClientConfiguration, QoS};
use esp_idf_svc::sys::EspError;
use esp_idf_svc::wifi::AuthMethod;
use crate::lcd;


const SSID: &str = "Wokwi-GUEST";
const PASSWORD: &str = "";


pub  fn connection_wifi()  {
    let sysloop = EspSystemEventLoop::take().unwrap(); //obter acesso ao systemloop
    let nvs = EspDefaultNvsPartition::take().unwrap(); //obter acesso a Non-Volatile Storage, necessário para o driver Wifi


    let pref = Peripherals::take().unwrap(); //obter acesso perifericos

    let mut led = PinDriver::output(pref.pins.gpio2).unwrap();

    let mut wifi_driver = EspWifi::new(pref.modem, sysloop, Some(nvs)).unwrap();


    // cria uma nova conexão
    wifi_driver
        .set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            password: PASSWORD.try_into().unwrap(),
            auth_method: AuthMethod::None,
            ..Default::default()
        })).unwrap();

    wifi_driver.start().unwrap();
    println!("Wifi Iniciado? : {:?}", wifi_driver.is_started());
    println!("Wifi Conectando... {:?}", wifi_driver.connect());

    let mut c =0;
    loop {
        c+=1;
        println!("Tentativa de Conexão #{c}");
        let res = wifi_driver.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;// sai do loop e vai para próximo passo
                }
            }
            Err(err) => {
                println!("{:?}", err);
                loop {}
            }
        }
        FreeRtos::delay_ms(1000u32);//delay antes de proxima verificacao de conexao
    }
    println!("{:?}", wifi_driver.is_connected());



    c=0;
    loop {
        c+=1;
        println!("Tentativa de obter IP do DHCP #{c}");
        let res = wifi_driver.is_up();
        match res {
            Ok(connected) => {
                if connected {
                    let ip =wifi_driver.sta_netif().get_ip_info();
                    println!("IP criado. {:?}", ip);
                    led.set_high().unwrap(); //liga LED para indicar wifi conectada
                    break;
                }
            }
            Err(err) => {
                println!("{:?}", err);
                loop {}
            }
        }
        FreeRtos::delay_ms(1000u32);
    }



    let mqtt_config = MqttClientConfiguration::default();

    let mqtt_url = "mqtt://broker.hivemq.com";


    let mut client = EspMqttClient::new_cb(
        mqtt_url,
        &mqtt_config,
        move |message_event| {
            match message_event.payload(){
                EventPayload::Connected(_) => {
                    println!("Conectado a {mqtt_url}");
                },
                EventPayload::Subscribed(id) => println!("Inscrito com id {id}"),



                EventPayload::Received{data, ..} => {
                    if !data.is_empty() {
                        led.toggle().unwrap();
                        println!("Mensagem recebida:  {}", std::str::from_utf8(data).unwrap());
                        FreeRtos::delay_ms(500u32);
                        led.toggle().unwrap();
                    }
                }
                EventPayload::Error(_) => {
                    println!("Erro conectando a {mqtt_url}!")

                }
                _ =>  {}
            };
        },
    ).unwrap();


    client.subscribe("profrs/led",QoS::AtLeastOnce).expect("erro ao subscrever no tópico!");
    println!("Esperando mensagem...");




    //TEMPERATURA
    let pin15 = pref.pins.gpio15;
    let mut sensor = gpio::PinDriver::input_output_od(pin15).unwrap();
    sensor.set_high().unwrap();
    let mut d = delay::Ets;

    let mut  cc = 0;




    //LCD
    unsafe {
        lcd::init(16, 32, 33, 14, 27, 26, 25);

    }

    let line_1 = "  Hello Wokwi!";

    //LOOP PRAGRAM
    loop {
        cc+= 1;


        match dht22::read(&mut d, &mut sensor) {
            Ok(r) => {

                let json =      format!(  "{{ \"humidity\": {},  \"temperature\" : {}  }}"  , r.relative_humidity , r.temperature);

                send_data(&mut client,"profrs/led".to_string(),json);

                unsafe {
                    lcd::text(format! ("TEMP  {} C°",r.temperature).to_string());
                    lcd::set_cursor(0,1);
                    lcd::text(format! ("UMI   {}",r.relative_humidity).to_string());

                }
            },
            Err(e) => println!("Failed with error: {:?}", e),
        }


        FreeRtos::delay_ms(3000u32);

        unsafe {
            lcd::clear();
        }

    }

}


 fn send_data( client:&mut EspMqttClient,topic:String , payload:String){
     client.publish(topic.as_str(),QoS::AtLeastOnce,false,payload.as_bytes()).unwrap();
 }


