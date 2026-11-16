#include <WiFi.h>

int LED_PIN = 2;

int VIBRATION_INPUT_PIN = 23;

const int WINDOW_TIME_SECONDS = 1;
const int SAMPLE_FREQUENCY_HZ = 20;

const int PERIOD = 1 / SAMPLE_FREQUENCY_HZ;

const int SAMPLE_WINDOW = WINDOW_TIME_SECONDS * SAMPLE_FREQUENCY_HZ;

const int THRESHOLD_PERCENT = 30; //complete guess highkey

//think of it as a fixed sized queue keeping track of the last SAMPLE_WINDOW samples.
bool samples[SAMPLE_WINDOW]; 
int ring_buffer_index = 0; 
int high_samples = 0;

const char* SSID = "INSERT_SSID";
const char* WIFI_PASSWORD = "INSERT_WIFI_PASSWORD";

void setup() {
  Serial.begin(115200);
  pinMode(LED_PIN, OUTPUT);
  pinMode(VIBRATION_INPUT_PIN, INPUT);

  for (int i = 0; i <= SAMPLE_WINDOW; i++) {
    samples[i] = false;
  }
}

void loop() {
  delay(PERIOD);

  if (samples[ring_buffer_index] == true) {
    high_samples--;
  }

  if( digitalRead(VIBRATION_INPUT_PIN) == HIGH) {
    digitalWrite(LED_PIN, HIGH);
    samples[ring_buffer_index] = true;
    high_samples++;    
  }else {
    digitalWrite(LED_PIN, LOW);
    samples[ring_buffer_index] = false;
  }


  int percent_high = high_samples*100 / SAMPLE_WINDOW;
  if (percent_high >= THRESHOLD_PERCENT) {
    Serial.printf("%d, | you be washing\n", percent_high);
  }else{
    Serial.printf("%d | no wash\n", percent_high);
  }

  ring_buffer_index = (ring_buffer_index + 1) % SAMPLE_WINDOW;
}



void initialiseWifi() {
  Wifi.mode(WIFI_STA);
  WiFi.begin(SSID, WIFI_PASSWORD);

  Serial.println("Connecting to WiFi ..");
  while (WiFi.status() != WL_CONNECTED) {
    Serial.print('.');
    delay(1000);
  }

  Serial.printf("WiFi Connected Successfully to: ")
  Serial.println(WiFi.localIP());
}
