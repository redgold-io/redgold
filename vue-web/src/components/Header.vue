<template>

  <div class="container-fluid">
  <b-navbar class="custom-navbar">
    <b-img :src="require('@/assets/logo.png')" alt="Logo" class="logo"></b-img>
<!--    <b-navbar-brand class="brand">Redgold Explorer</b-navbar-brand>-->
    <router-link to="/" class="brand">
      Redgold Explorer
    </router-link>
<!--    <b-navbar-nav class="ml-auto">-->
<!--      <b-nav-item><h6>Home</h6></b-nav-item>-->
<!--    </b-navbar-nav>-->
  </b-navbar>

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">
        <!-- Search bar -->
        <b-jumbotron class="search-bar py-4">
          <h6 class="text-light">Enter hash search query:</h6>
          <b-form inline>
            <label class="sr-only" for="inline-form-input-name">Search</label>
            <b-form-input id="inline-form-input-name" class="search-input" placeholder="Query hash..." />
            <b-button variant="primary">Submit</b-button>
          </b-form>
        </b-jumbotron>
      </div>
    </div>
  </div>

</template>

<script>
import {BNavbar, BImg, BForm, BButton, BFormInput, BJumbotron} from 'bootstrap-vue'
//BNavbarBrand, BNavbarNav, BNavItem
export default {
  name: 'Header',
  components: {
    BJumbotron, BFormInput, BButton, BForm,
    BNavbar,
    // BNavbarBrand,
    // BNavbarNav,
    // BNavItem,
    BImg
  }
}
</script>

<style scoped>
.custom-navbar {
  background-color: #000;
}

.logo {
  height: 50px; /* adjust as needed */
  width: 50px; /* adjust as needed */
  margin-right: 10px;
}

.custom-navbar .navbar-brand {
  color: #fff;
  user-select: none;
  text-decoration: none;  /* removes underline */
}

.custom-navbar .navbar-brand:hover {
  background-color: #000;
  color: #fff;
}

.custom-navbar .nav-item h6 {
  color: #fff;
}

/* Add these lines to change link color */
.custom-navbar .brand {
  color: #fff;
}

.custom-navbar .brand:hover {
  text-decoration: none;  /* removes underline on hover */
  background-color: rgba(255, 255, 255, 0.1);  /* changes background color on hover */
}


.search-bar {
  background-color: #000;
}

.search-input, .search-input:focus {
  box-sizing: border-box; /* Include padding and border in element's total width and height */
  min-width: 600px;  /* adjust as needed */
  background-color: #191a19;
  color: #fff; /* white */
  /* outline: none; (optional, removes the default browser outline around the input when focused) */
}

</style>
