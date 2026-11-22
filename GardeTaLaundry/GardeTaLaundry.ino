#include <WiFi.h>
#include <HTTPClient.h>
#include <format>

int LED_PIN = 2;

int VIBRATION_INPUT_PIN = 32;

const int WINDOW_TIME_SECONDS = 1;
const int SAMPLE_FREQUENCY_HZ = 20;

const int PERIOD = 1000 / SAMPLE_FREQUENCY_HZ;

const int SAMPLE_WINDOW = WINDOW_TIME_SECONDS * SAMPLE_FREQUENCY_HZ;

const int THRESHOLD_PERCENT = 30; //complete guess highkey

//think of it as a fixed sized queue keeping track of the last SAMPLE_WINDOW samples.
bool samples[SAMPLE_WINDOW];
int ring_buffer_index = 0;
int high_samples = 0;

const char* SSID = "Magd-IoT";
// const char* WIFI_PASSWORD = "INSERT_WIFI_PASSWORD";

const int REPORTING_INTERVAl_SECONDS = 10; //also arbitrary

const char* ENDPOINT = "http://10.176.103.64:3000/set";

constexpr std::string_view WASHER_ID = "THOMPSONS_FLOOR0_WASH";

WiFiClient client;
HTTPClient http;

void setup() {
  Serial.begin(115200);
  pinMode(LED_PIN, OUTPUT);
  pinMode(VIBRATION_INPUT_PIN, INPUT);

  for (int i = 0; i < SAMPLE_WINDOW; i++) {
    samples[i] = false;
  }

  initialiseWifi();
}

void loop() {
  delay(PERIOD); //need to tweak this so the frequency is actually the frequency

  bool washerOn = machineOn();

  if (WiFi.status() == WL_CONNECTED) {
    
    constexpr std::string_view requestTemplate = "{{"
      "\"machineId\": \"{}\", "
      "\"isOn\": {}" 	 
    "}}";

    std::string requestData = std::format(requestTemplate, WASHER_ID, (washerOn ? "true" : "false"));

    Serial.printf(requestData.c_str());

    http.addHeader("Content-Type", "application/json");
    int httpResponseCode = http.POST(String(requestData.c_str()));
    
    // int httpResponseCode = http.GET();

    Serial.printf("HTTP response: %d\n", httpResponseCode);

    if (httpResponseCode <= 0) {
      Serial.printf("Error connecting to server, HTTP response: %d", httpResponseCode);
      initialiseWifi();
    }

  }else{
    //Reconnect
    Serial.printf("Wifi Disconnected!\n");
    initialiseWifi();
  }
}

void initialiseWifi() {
  Serial.printf("Initialising Wifi...");
  WiFi.mode(WIFI_STA);
  WiFi.begin(SSID);

  int wifiWait = 0;
  while (WiFi.status() != WL_CONNECTED) {
    Serial.printf("Connecting to WiFi for %d seconds\n", wifiWait);
    wifiWait++;
    delay(1000);
  }

  Serial.printf("WiFi Connected Successfully to: ");
  // Serial.printf(WiFi.localIP()); 

  http.end(); //Clean up any existing connections
  http.begin(client, ENDPOINT);
  http.setReuse(true); //keep it alive
  return;
}

bool machineOn() {
  if (samples[ring_buffer_index] == true) {
    high_samples--;
  }

  if( digitalRead(VIBRATION_INPUT_PIN) == HIGH) {
    Serial.printf("high\n");
    digitalWrite(LED_PIN, HIGH);
    samples[ring_buffer_index] = true;
    high_samples++;
  }else {
    Serial.printf("low\n");
    digitalWrite(LED_PIN, LOW);
    samples[ring_buffer_index] = false;
  }

  ring_buffer_index = (ring_buffer_index + 1) % SAMPLE_WINDOW;

  int percent_high = high_samples*100 / SAMPLE_WINDOW;
  if (percent_high >= THRESHOLD_PERCENT) {
    Serial.printf("%d, | you be washing\n", percent_high);
    return true;
  }else{
    Serial.printf("%d | no wash\n", percent_high);
    return false;
  }

}



