#include <WiFi.h>
#include <HTTPClient.h>
#include <format>

int LED_PIN = 2;

const int WINDOW_TIME_SECONDS = 1;
const int SAMPLE_FREQUENCY_HZ = 20;

const int PERIOD = 1000 / SAMPLE_FREQUENCY_HZ;

const int SAMPLE_WINDOW = WINDOW_TIME_SECONDS * SAMPLE_FREQUENCY_HZ;

const int THRESHOLD_PERCENT = 30; //complete guess highkey

const char* SSID = "Magd-IoT";
// const char* WIFI_PASSWORD = "INSERT_WIFI_PASSWORD";

const int REPORTING_INTERVAl_SECONDS = 10; //also arbitrary

const char* ENDPOINT = "http://10.176.103.64:3000/set";

constexpr std::string_view WASHER_ID = "THOMPSONS_FLOOR0_WASH";
constexpr std::string_view DRYER_ID = "THOMPSONS_FLOOR0_DRY";


WiFiClient client;
HTTPClient http;

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

class Sensor {
  private:
    bool samples[SAMPLE_WINDOW];
    int ringBufferIndex = 0;
    int highSamples = 0;
    int vibrationInput;
    std::string_view machineId;
  public:
    Sensor(int p, std::string_view id) {
      for (int i = 0; i < SAMPLE_WINDOW; i++) {
        samples[i] = false;
      }

      vibrationInput = p;
      machineId = id;

      pinMode(vibrationInput, INPUT);
    }

    bool machineOn() {
      if (samples[ringBufferIndex] == true) {
        highSamples--;
      }

      if( digitalRead(vibrationInput) == HIGH) {
        Serial.printf("high\n");
        digitalWrite(LED_PIN, HIGH);
        samples[ringBufferIndex] = true;
        highSamples++;
      }else {
        Serial.printf("low\n");
        digitalWrite(LED_PIN, LOW);
        samples[ringBufferIndex] = false;
      }

      ringBufferIndex = (ringBufferIndex + 1) % SAMPLE_WINDOW;

      int percent_high = highSamples*100 / SAMPLE_WINDOW;
      if (percent_high >= THRESHOLD_PERCENT) {
        Serial.printf("%d, | you be washing\n", percent_high);
        return true;
      }else{
        Serial.printf("%d | no wash\n", percent_high);
        return false;
      }
    }

  void reportToServer() {
    if (WiFi.status() == WL_CONNECTED) {
        
        constexpr std::string_view requestTemplate = "{{"
          "\"machineId\": \"{}\", "
          "\"isOn\": {}" 	 
        "}}";

        const char* state = machineOn() ? "true" : "false";
        std::string requestData = std::vformat(
            requestTemplate,
            std::make_format_args(machineId, state));

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

};

//think of it as a fixed sized queue keeping track of the last SAMPLE_WINDOW samples.

Sensor washerSensor = Sensor(32, WASHER_ID);
Sensor dryerSensor = Sensor(33, DRYER_ID);

void setup() {
  Serial.begin(115200);
  pinMode(LED_PIN, OUTPUT);





  initialiseWifi();
}

void loop() {
  delay(PERIOD); //need to tweak this so the frequency is actually the frequency

  washerSensor.reportToServer();
  dryerSensor.reportToServer();
}







