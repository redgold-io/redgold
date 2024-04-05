<template>
  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10 d-flex flex-column align-items-center content-center">

        <h4>Faucet Request</h4>
        <div class="jumbotron search-bar py-4">
          <h5 class="text-light">Funds will be sent to this address:</h5>
          <form class="d-flex align-items-center demo-form">
            <!--            <label for="inline-form-input-name" class="sr-only">Search</label>-->
            <input v-model="searchValue" id="inline-form-input-name" class="form-control mr-2 search-input" type="text" placeholder="Your Address...">
<!--            <button type="submit" class="btn btn-primary">Submit</button>-->
            <button class="g-recaptcha btn btn-primary"
                    data-sitekey="6Lc0yocpAAAAANe4pUl9A1-akxaHaLcE4FBYCvIV"
                    data-callback='onSubmit'
                    data-action='submit'>Submit</button>
          </form>
        </div>


        <div>
          <h3 v-if="errorMessage.length > 0">{{this.errorMessage}}</h3>
        </div>
        <div>
          <h3 v-if="loading">Awaiting transaction completion (~30 seconds)...</h3>
        </div>
        <div>
          <h4 v-if="this.successTransactionHash.length > 1">Success! View transaction below:</h4>
        </div>

        <div>
          <HashLink v-if="this.successTransactionHash.length > 1" :data="this.successTransactionHash" :shorten="false"/>
        </div>

      </div>

      <!-- Buffer div -->
      <div class="col-1"></div>

    </div>
  </div>
</template>


<script>



import axios from "axios";
import fetchHashInfo from "@/components/mixins/fetchHashInfo";
import HashLink from "@/components/util/HashLink.vue";

export default {
  name: 'FaucetRequest',
  components: {
    HashLink

  },
  mixins: [fetchHashInfo],
  data() {
    return {
      searchValue: '',
      successTransactionHash: '1',
      errorMessage: '',
      loading: false,
    }
  },
  mounted() {
    window.onSubmit = this.onSubmit;
    this.loadRecaptchaScript();
  },
  beforeUnmount() {
    this.removeRecaptchaScript();
    delete window.onSubmit; // Clean up to avoid leaks
  },
  methods: {
    loadRecaptchaScript() {
      if (document.getElementById('recaptchaScript')) return; // Script already loaded

      const script = document.createElement('script');
      script.id = 'recaptchaScript';
      script.src = 'https://www.google.com/recaptcha/api.js';
      document.head.appendChild(script);
    },
    removeRecaptchaScript() {
      const script = document.getElementById('recaptchaScript');
      if (script) {
        document.head.removeChild(script);
      }
    },
    onSubmit(token) {
      this.loading = true;
      this.successTransactionHash = '1';
      // You might want to do something with the token or directly submit the form
      // document.getElementById("demo-form").submit();
      console.log('token for faucet submit:', token);

      let url = this.getUrl();
      let full = `${url}/explorer/faucet/${this.searchValue}?token=${token}`
      console.log('full url:', full);
      axios.get(full)
          .then(response => {
            this.loading = false;
            let data = response.data;
            console.log("Response data", data); // log the response data
            let datum = data['transaction_hash'];
            console.log("datum", datum);
            if (datum != null) {
              this.successTransactionHash = datum;
            } else {
              let msg = data['message'];
              if (msg != null) {
                this.errorMessage = msg;
              } else {
                this.errorMessage = data.toString()
              }

            }
          })
          .catch(error => {
            this.loading = false;
            console.error(error);
          });

    }
  }
}
</script>

<style>

h4 {
  margin-top: 20px;
  margin-bottom: 20px;
}

.search-bar {
  background-color: #000;
}

.search-input,
.search-input:focus {
  box-sizing: border-box;
  min-width: 600px;
  max-width: 600px;
  background-color: #191a19;
  color: #fff;
}
.search-input::placeholder {
  color: #ccc;
}

.content-center {
  margin: 0 auto;
  text-align: center;
}

</style>